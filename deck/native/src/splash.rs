//! The splash (S3, owner 2026-10-29: "das scutoons logo gross in weiss"): the official SCUTOONS logo
//! (viewer/static/SCUTOONS-logo.svg, its fill turned white – assets/SCUTOONS-logo-white.svg) big on black, over the
//! stage while it loads (the first frame, the chapter, the music) and while the browser starts after F2. The same SVG
//! is the boot splash (plymouth) and the BIOS menu's background (deck/os, rendered at build time).
use anyhow::{Context, Result};

pub const LOGO_SVG: &[u8] = include_bytes!("../assets/SCUTOONS-logo-white.svg");

/// the logo rasterized `width` px wide: straight RGBA, the height by the logo's aspect
pub fn logo(width: u32) -> Result<(u32, u32, Vec<u8>)> {
    let tree = resvg::usvg::Tree::from_data(LOGO_SVG, &resvg::usvg::Options::default()).context("the logo SVG")?;
    let size = tree.size();
    let k = width.max(1) as f32 / size.width();
    let h = ((size.height() * k).ceil() as u32).max(1);
    let mut pm = resvg::tiny_skia::Pixmap::new(width.max(1), h).context("a pixmap for the logo")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pm.as_mut());
    let mut px = pm.take();
    // tiny-skia is premultiplied; the passes blend straight alpha
    for p in px.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Ok((width.max(1), h, px))
}

/// The splash's visibility: it fades in fast (F2) and out gently (loaded), 0 = gone, 1 = only the logo.
#[derive(Debug, Clone, Copy)]
pub struct Fade {
    pub alpha: f32,
}

impl Fade {
    pub fn step(&mut self, shown: bool, dt: f32) {
        let rate = if shown { 1.0 / 0.15 } else { 1.0 / 0.6 };
        let target = if shown { 1.0 } else { 0.0 };
        let d = (target - self.alpha).clamp(-rate * dt, rate * dt);
        self.alpha = (self.alpha + d).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_renders_white() {
        let (w, h, px) = logo(492).unwrap();
        assert_eq!((w, h), (492, 193));
        let opaque = px.chunks_exact(4).filter(|p| p[3] == 255).count();
        assert!(opaque > (w * h / 5) as usize, "{opaque} opaque pixels");
        assert!(px.chunks_exact(4).filter(|p| p[3] == 255).all(|p| p[..3] == [255, 255, 255]));
    }

    #[test]
    fn the_fade_goes_in_fast_and_out_slow() {
        let mut f = Fade { alpha: 0.0 };
        f.step(true, 0.1);
        assert!(f.alpha > 0.6);
        f.step(true, 0.1);
        assert_eq!(f.alpha, 1.0);
        f.step(false, 0.3);
        assert!((f.alpha - 0.5).abs() < 1e-4);
    }
}
