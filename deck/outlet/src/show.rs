//! Show media and the deck token (#470; server/docs/API.md "Decks", `GET /decks/media`): which preload `url`s are show
//! media in the vault and how a deck gets them – the same rule on the native deck and in the browser.
//!
//! - A preload `url` of the form `live/<toon>/<file>` (relative – the vault's path) is a **show medium**: the deck asks the
//!   API `GET <api>/decks/media?path=…` with its token (`Authorization: Bearer scd_…`) and fetches the short signed link it
//!   answers – **without** the token.
//! - Every other `url` (an absolute public URL, the library's slices) is fetched as it is, **never with the token**: a cue
//!   must not be able to hand the deck's token to another host. The token only ever goes to the API.
//!
//! The server checks the path again (`ProtectedFiles::SHOW_PATH`); this check only keeps a deck from asking for nonsense.

use serde_json::Value;

/// The show medium's vault path when `url` names one (`live/<toon>/<file>`, at most five parts below the toon, no `.` /
/// `..`, the characters the server allows), else None (fetch the url as it is, without the token)
pub fn show_path(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("live/")?;
    let mut parts = rest.split('/');
    let toon = parts.next()?;
    let toon_ok = (1..=64).contains(&toon.len()) && toon.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    let below: Vec<&str> = parts.collect();
    let part_ok = |p: &&str| {
        (1..=128).contains(&p.len())
            && !p.starts_with('.')
            && p.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    };
    (toon_ok && (1..=5).contains(&below.len()) && below.iter().all(part_ok) && !url.contains("..")).then_some(url)
}

/// The API request for a show medium's signed link: `<api>/decks/media?path=<path>` (the path's characters need no escaping
/// – `show_path` allows only `[A-Za-z0-9_.-/]`)
pub fn signed_link_request(api: &str, path: &str) -> String {
    format!("{}/decks/media?path={path}", api.trim_end_matches('/'))
}

/// The API's answer (`{ url, expires }`, or `{ error }` with the status) → the signed link, or the reason it is missing
/// (the HUD's and `media.ready`'s `why`)
pub fn signed_link(status: u16, body: &Value) -> Result<String, String> {
    match status {
        200 => body["url"].as_str().filter(|u| u.starts_with("https://") || u.starts_with("http://")).map(str::to_string).ok_or_else(|| "the API gave no link".into()),
        401 => Err("show medium: the deck is not paired (or its approval ended)".into()),
        403 => Err("show medium: only a paired deck's own token".into()),
        _ => Err(format!("show medium: HTTP {status} {}", body["error"].as_str().unwrap_or("")).trim_end().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_vault_paths_are_show_media() {
        for ok in ["live/catpope/choir.ogg", "live/cat-pope_2/stems/a/b/c/d.wav", "live/x/v1.0.mp4"] {
            assert_eq!(show_path(ok), Some(ok), "{ok}");
        }
        for no in [
            "https://cdn.scutoons.com/live/catpope/x.ogg", "/live/catpope/x.ogg", "live/catpope", "live/catpope/", "live/Cat/x.ogg",
            "live/catpope/../scores/x.pdf", "live/catpope/.hidden", "live//x.ogg", "live/catpope/a/b/c/d/e/f.ogg", "live/catpope/x y.ogg",
            "live/catpope/x.ogg?token=1", "scores/x/y.pdf", "",
        ] {
            assert_eq!(show_path(no), None, "{no}");
        }
    }

    #[test]
    fn the_signed_link_comes_from_the_api() {
        assert_eq!(signed_link_request("https://api.scutoons.com/api/", "live/catpope/x.ogg"), "https://api.scutoons.com/api/decks/media?path=live/catpope/x.ogg");
        assert_eq!(signed_link(200, &json!({ "url": "https://vault.test/live/catpope/x.ogg?token=t", "expires": 1 })), Ok("https://vault.test/live/catpope/x.ogg?token=t".into()));
        assert!(signed_link(200, &json!({ "url": "javascript:alert(1)" })).is_err());
        assert!(signed_link(401, &json!({})).unwrap_err().contains("not paired"));
        assert_eq!(signed_link(400, &json!({ "error": "invalid_path" })), Err("show medium: HTTP 400 invalid_path".into()));
        assert_eq!(signed_link(502, &json!(null)), Err("show medium: HTTP 502".into()));
    }
}
