//! The deck's pairing as the menu shows it (#180, CYBERDECKS.md §9; #386: the corner card moved into the menu's "Pair
//! this deck" page – owner 2026-11-11: "Die pairing karte muss ins haupt menu"): while the deck waits for a member, the
//! code to pair with and the QR of the pairing link (`/account/decks?deck=<code>` on the viewer – scan it with the phone,
//! signed in, approve). What it says comes from deck-link's daemon (`deck_link::pairing`, /run/deck-link/state – polled,
//! never on the frame's critical path) on the native deck, from the browser deck's own link (space-web `deck_link`).
//! Here the state becomes the menu's plain data (deck-ui `menu::Pair`); the menu draws it.
use deck_link::pairing::{format_code, pairing_url, Shown};
use deck_ui::menu::{Pair, Qr};
use qrcodegen::{QrCode, QrCodeEcc};

/// The menu's pairing for deck-link's state (none: no pairing on this deck); `app_url` = the viewer the member pairs on.
pub fn of(state: Option<&Shown>, app_url: &str) -> Pair {
    match state {
        None => Pair::Off,
        Some(Shown::Pending { code }) => waiting(code, app_url),
        Some(Shown::Ready { name }) => Pair::Paired(name.clone()),
        Some(Shown::Offline { reason }) => Pair::Offline(reason.clone()),
    }
}

/// Waiting for a member with `code`: the code as people type it (K7Q-42M), the link to type without its scheme and
/// query, the QR of the link.
pub fn waiting(code: &str, app_url: &str) -> Pair {
    let url = pairing_url(app_url, code);
    let hint = url.split_once("://").map_or(url.as_str(), |(_, rest)| rest);
    let hint = hint.split_once('?').map_or(hint, |(path, _)| path).to_string();
    Pair::Waiting { code: format_code(code), hint, qr: qr(&url) }
}

/// `text`'s QR modules (an empty one when it does not fit a code)
pub fn qr(text: &str) -> Qr {
    let Ok(q) = QrCode::encode_text(text, QrCodeEcc::Medium) else { return Qr::default() };
    let n = q.size() as usize;
    Qr { n, dark: (0..n * n).map(|i| q.get_module((i % n) as i32, (i / n) as i32)).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_shows_the_code_while_the_deck_waits_for_a_member() {
        let Pair::Waiting { code, hint, qr } = of(Some(&Shown::Pending { code: "K7Q42M".into() }), "https://scutoons.com/") else { panic!("waiting") };
        assert_eq!((code.as_str(), hint.as_str()), ("K7Q-42M", "scutoons.com/account/decks"));
        // the link's QR (version 4 for this link, 33 modules): the top left finder pattern at its corner
        assert_eq!(qr.n, 33);
        assert_eq!((qr.dark[0], qr.dark[qr.n + 1], qr.dark[2 * qr.n + 2]), (true, false, true));
        assert_eq!(of(Some(&Shown::Ready { name: "X1".into() }), "https://scutoons.com"), Pair::Paired("X1".into()));
        assert_eq!(of(Some(&Shown::Offline { reason: "no network".into() }), "https://scutoons.com"), Pair::Offline("no network".into()));
        assert_eq!(of(None, "https://scutoons.com"), Pair::Off);
    }
}
