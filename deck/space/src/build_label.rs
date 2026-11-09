//! #423: the build label – the build's version (`crate::BUILD`, e.g. `0.1.0.r3093.g8eccc753`, what the update bundle carries)
//! drawn small in the TOP RIGHT corner of the window, always: every scene, every HUD mode (F1 Full / Bar / Off – it is not
//! part of the HUD), kiosk fullscreen too. So the owner sees whether an update really arrived. What it says and where it
//! sits is deck-ui's (`build_label`, `label_rect`); this file rasters the text on the CPU like the HUD (font8x8, no font
//! files) into a small RGBA image that render.rs uploads once and draws at the HUD's integer scale.
use crate::hud::glyphs;

/// the text's colour and the panel behind it: semi-transparent dark, so the text reads on the cathedral's bright and dark scenes
const TEXT: [u8; 4] = [240, 240, 240, 235];
const PANEL: [u8; 4] = [0, 0, 0, 120];
/// padding around the text (px at the raster's scale)
const PAD_X: u32 = 4;
const PAD_Y: u32 = 2;

/// the label for `build`, rastered: (width, height, RGBA)
pub fn raster(build: &str) -> (u32, u32, Vec<u8>) {
    let text = deck_ui::build_label(build);
    let w = text.chars().count() as u32 * 8 + 2 * PAD_X;
    let h = 8 + 2 * PAD_Y;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for p in px.chunks_exact_mut(4) {
        p.copy_from_slice(&PANEL);
    }
    let mut put = |x: u32, y: u32, c: [u8; 4]| {
        if x < w && y < h {
            let i = ((y * w + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&c);
        }
    };
    glyphs(&mut put, PAD_X, PAD_Y, &text, TEXT);
    (w, h, px)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the label on screen says exactly what the build constant says (and so what the update bundle carries)
    #[test]
    fn the_label_text_equals_the_build_const() {
        assert!(!crate::BUILD.is_empty());
        assert_eq!(deck_ui::build_label(crate::BUILD), crate::BUILD, "an ASCII build string of at most {} characters is shown whole", deck_ui::LABEL_COLS);
    }

    #[test]
    fn the_raster_holds_the_text_on_its_panel() {
        let (w, h, px) = raster("0.1.0.r3093.g8eccc753");
        assert_eq!((w, h), (21 * 8 + 8, 12));
        assert_eq!(px.len(), (w * h * 4) as usize);
        let lit = px.chunks_exact(4).filter(|p| *p == TEXT).count();
        let panel = px.chunks_exact(4).filter(|p| *p == PANEL).count();
        assert!(lit > 100, "text pixels: {lit}");
        assert!(panel > lit, "the panel shows around the text");
        // semi-transparent: no pixel is opaque
        assert!(px.chunks_exact(4).all(|p| p[3] < 255));
        // the text's first column starts after the padding: nothing lit left of it
        assert!(px.chunks_exact(4).enumerate().filter(|(_, p)| *p == TEXT).all(|(i, _)| i as u32 % w >= PAD_X));
    }
}
