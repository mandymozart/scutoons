//! The pairing card (#180, CYBERDECKS.md §9): while the deck waits for a member, the stage shows the code to pair with
//! and the QR of the pairing link (`/account/decks?deck=<code>` on the viewer – scan it with the phone, signed in,
//! approve) in its lower right corner, over the splash and the scene, until the member approves it. What it says comes
//! from deck-link's daemon (`deck_link::pairing`, /run/deck-link/state – polled, never on the frame's critical path);
//! no daemon, paired or offline = no card. Built-in scenes play before approval.
//! Rastered on the CPU like the HUD (font8x8 + the QR's modules into a small RGBA image), drawn by render.rs at an
//! integer scale – 2 at 1080p, so a QR module is 10 px on the screen.
use deck_link::pairing::{format_code, pairing_url, Shown};
use font8x8::UnicodeFonts;
use qrcodegen::{QrCode, QrCodeEcc};

pub const W: u32 = 232;
const PAD: u32 = 8;
/// the QR's white square (the quiet zone inside it)
const QR_BOX: u32 = W - 2 * PAD;
const Y_TITLE: u32 = PAD;
const Y_CODE: u32 = Y_TITLE + 8 + 6;
const Y_QR: u32 = Y_CODE + 16 + 8;
const Y_HINT: u32 = Y_QR + QR_BOX + 6;
pub const H: u32 = Y_HINT + 8 + PAD;

/// What the card says (the state → text, tested without a GPU)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub title: String,
    /// the code as people type it: K7Q-42M
    pub code: String,
    /// what the QR opens
    pub url: String,
    /// the same link to type, without the scheme
    pub hint: String,
}

impl Card {
    /// The card for deck-link's state: Some only while the deck waits for a member
    pub fn of(state: Option<&Shown>, app_url: &str) -> Option<Card> {
        let Some(Shown::Pending { code }) = state else { return None };
        let url = pairing_url(app_url, code);
        let hint = url.split_once("://").map_or(url.as_str(), |(_, rest)| rest);
        let hint = hint.split_once('?').map_or(hint, |(path, _)| path).to_string();
        Some(Card { title: "Pair this deck".into(), code: format_code(code), url, hint })
    }

    /// The card as straight RGBA, W × H: a dark panel, the title, the code twice as big, the QR (black on white), the link
    pub fn raster(&self) -> Vec<u8> {
        let mut px = vec![0u8; (W * H * 4) as usize];
        for p in px.chunks_exact_mut(4) {
            p.copy_from_slice(&[17, 17, 17, 235]);
        }
        let white = [240, 240, 240, 255];
        text(&mut px, center(&self.title, 1), Y_TITLE, &self.title, 1, white);
        text(&mut px, center(&self.code, 2), Y_CODE, &self.code, 2, [127, 255, 221, 255]);
        fill(&mut px, PAD, Y_QR, QR_BOX, QR_BOX, [255, 255, 255, 255]);
        if let Ok(qr) = QrCode::encode_text(&self.url, QrCodeEcc::Medium) {
            let n = qr.size() as u32;
            // whole pixels per module, at least the 4-module quiet zone around it inside the box
            let m = (QR_BOX / (n + 8)).max(1);
            let off = (QR_BOX - n * m) / 2;
            for y in 0..n {
                for x in 0..n {
                    if qr.get_module(x as i32, y as i32) {
                        fill(&mut px, PAD + off + x * m, Y_QR + off + y * m, m, m, [0, 0, 0, 255]);
                    }
                }
            }
        }
        text(&mut px, center(&self.hint, 1), Y_HINT, &self.hint, 1, white);
        px
    }
}

fn center(s: &str, scale: u32) -> u32 {
    let w = s.chars().count() as u32 * 8 * scale;
    W.saturating_sub(w) / 2
}

fn text(px: &mut [u8], x: u32, y: u32, s: &str, scale: u32, c: [u8; 4]) {
    for (i, ch) in s.chars().enumerate() {
        let glyph = font8x8::BASIC_FONTS.get(ch).or_else(|| font8x8::BASIC_FONTS.get('?')).unwrap_or([0; 8]);
        for (gy, bits) in glyph.iter().enumerate() {
            for gx in 0..8 {
                if bits & (1 << gx) != 0 {
                    fill(px, x + (i as u32 * 8 + gx) * scale, y + gy as u32 * scale, scale, scale, c);
                }
            }
        }
    }
}

fn fill(px: &mut [u8], x: u32, y: u32, w: u32, h: u32, c: [u8; 4]) {
    for yy in y..(y + h).min(H) {
        for xx in x..(x + w).min(W) {
            let i = ((yy * W + xx) * 4) as usize;
            px[i..i + 4].copy_from_slice(&c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_shows_the_code_while_the_deck_waits_for_a_member() {
        let pending = Shown::Pending { code: "K7Q42M".into() };
        let card = Card::of(Some(&pending), "https://scutoons.com/").unwrap();
        assert_eq!(
            card,
            Card {
                title: "Pair this deck".into(),
                code: "K7Q-42M".into(),
                url: "https://scutoons.com/account/decks?deck=K7Q42M".into(),
                hint: "scutoons.com/account/decks".into(),
            }
        );
        // paired, offline, no daemon: no card
        assert_eq!(Card::of(Some(&Shown::Ready { name: "X1".into() }), "https://scutoons.com"), None);
        assert_eq!(Card::of(Some(&Shown::Offline { reason: "no network".into() }), "https://scutoons.com"), None);
        assert_eq!(Card::of(None, "https://scutoons.com"), None);
    }

    #[test]
    fn the_card_carries_the_code_and_a_qr() {
        let card = Card::of(Some(&Shown::Pending { code: "K7Q42M".into() }), "https://scutoons.com").unwrap();
        let px = card.raster();
        assert_eq!(px.len(), (W * H * 4) as usize);
        let count = |c: [u8; 4]| px.chunks_exact(4).filter(|p| *p == c).count();
        assert!(count([127, 255, 221, 255]) > 100, "the code's pixels");
        // the QR (version 4 for this link, 33 modules, 5 px a module): its dark modules inside the white box
        let qr = QrCode::encode_text(&card.url, QrCodeEcc::Medium).unwrap();
        let n = qr.size() as u32;
        let m = QR_BOX / (n + 8);
        assert!(m >= 4, "{m} px a module");
        let dark = count([0, 0, 0, 255]);
        assert!(dark > (n * n * m * m / 4) as usize && dark < (n * n * m * m) as usize, "{dark} dark pixels");
        assert!(count([255, 255, 255, 255]) > (QR_BOX * QR_BOX / 3) as usize);
        // the top left finder pattern sits at the box's corner + the quiet zone
        let off = (QR_BOX - n * m) / 2;
        let at = |x: u32, y: u32| {
            let i = (((Y_QR + off + y * m + m / 2) * W + PAD + off + x * m + m / 2) * 4) as usize;
            [px[i], px[i + 1], px[i + 2]]
        };
        assert_eq!((at(0, 0), at(1, 1), at(2, 2)), ([0, 0, 0], [255, 255, 255], [0, 0, 0]));
    }
}
