//! The deck's menu (#386) painted by the real code (`menu.rs`, `menu/draw.rs`, deck-kit's live components), to compare
//! with draft 1: one PNG per frame over a dark stand-in for the scene (the splash's logo is the stage's, not drawn here).
//!   cargo run -p deck-ui --example menu-frames -- <out dir>
use deck_ui::menu::{draw::Pixels, Facts, Input, Menu, Page, Pair};

const DECK: (f32, f32) = (1920.0, 1080.0);
const PHONE: (f32, f32) = (780.0, 1688.0);

/// the menu's picture over the stand-in, grown to the screen's pixels as the stage does (nearest)
fn frame(out: &str, name: &str, (w, h): (f32, f32), px: &Pixels) -> image::ImageResult<()> {
    let (w, h) = (w as u32, h as u32);
    let k = (w as f32 / px.w as f32).max(h as f32 / px.h as f32);
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        let under = [(20.0 + 30.0 * y as f32 / h as f32) as u8, 14, (40.0 + 40.0 * x as f32 / w as f32) as u8];
        let p = px.at(((x as f32 / k) as u32).min(px.w - 1), ((y as f32 / k) as u32).min(px.h - 1));
        let a = p[3] as f32 / 255.0;
        let c = |i: usize| (p[i] as f32 * a + under[i] as f32 * (1.0 - a)).round() as u8;
        image::Rgba([c(0), c(1), c(2), 255])
    });
    img.save(format!("{out}/{name}.png"))
}

fn paint(m: &mut Menu, f: &Facts, screen: (f32, f32)) -> Pixels {
    m.paint(f, screen).flatten().expect("a picture")
}

fn main() -> image::ImageResult<()> {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let session = Some("output C7 · tenor: anna @ CHOIR".to_string());
    let deck = Facts { native: true, live_page: true, scenes_page: true, settings_page: true, density: 1.0, session, who: "tenor: anna".into(), scene: "fireflies".into(), ..Facts::default() };
    // 1 · the splash of a deck in no session
    let mut m = Menu::new(false);
    let alone = Facts { session: None, loading: true, ..deck.clone() };
    frame(&out, "1-splash", DECK, &paint(&mut m, &alone, DECK))?;
    // 2 · the main menu over the scene
    let mut m = Menu::new(true);
    m.input(Input::Open, &deck);
    frame(&out, "2-main-menu", DECK, &paint(&mut m, &deck, DECK))?;
    // 3 · pair this deck, waiting for a member
    let waiting = Facts { pair: deck_space_like_waiting(), ..deck.clone() };
    m.direct(Page::Pair);
    frame(&out, "3-pair", DECK, &paint(&mut m, &waiting, DECK))?;
    let signed = Facts { pair: Pair::SignedIn, native: false, ..deck.clone() };
    frame(&out, "3b-pair-signed-in", DECK, &paint(&mut m, &signed, DECK))?;
    // 4 · a later page, empty
    m.direct(Page::Equip);
    frame(&out, "4-equip-later", DECK, &paint(&mut m, &deck, DECK))?;
    // 5 · the phone: a finger's rows
    let phone = Facts { native: false, scenes_page: false, settings_page: false, density: 2.0, top: 48.0, ..deck.clone() };
    let mut m = Menu::new(true);
    m.input(Input::Open, &phone);
    frame(&out, "5-phone-menu", PHONE, &paint(&mut m, &phone, PHONE))?;
    let mut m = Menu::new(true);
    m.input(Input::Open, &phone);
    m.direct(Page::Pair);
    frame(&out, "5b-phone-pair", PHONE, &paint(&mut m, &Facts { pair: deck_space_like_waiting(), ..phone.clone() }, PHONE))?;
    let mut m = Menu::new(false);
    frame(&out, "5c-phone-splash", PHONE, &paint(&mut m, &Facts { session: None, ..phone }, PHONE))
}

/// a waiting pairing with a QR-like pattern (deck-space `pairing::waiting` makes the real one from the link)
fn deck_space_like_waiting() -> Pair {
    let n = 33;
    let finder = |x: usize, y: usize, ox: usize, oy: usize| {
        let (dx, dy) = (x.wrapping_sub(ox), y.wrapping_sub(oy));
        (dx < 7 && dy < 7).then(|| dx == 0 || dy == 0 || dx == 6 || dy == 6 || (2..5).contains(&dx) && (2..5).contains(&dy))
    };
    let dark = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            finder(x, y, 0, 0).or(finder(x, y, n - 7, 0)).or(finder(x, y, 0, n - 7)).unwrap_or((x * 7 + y * 13 + x * y) % 5 < 2)
        })
        .collect();
    Pair::Waiting { code: "Q5H-RT2".into(), hint: "scutoons.com/account/decks".into(), qr: deck_ui::menu::Qr { n, dark } }
}
