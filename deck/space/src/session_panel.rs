//! The deck editor's Session panel as plain data (#531; agents/research/SCENE-EVENTS.md §6.7): the conductor's view of the
//! linked readers (linked.rs `Readers`, relay.rs `Client::readers`) and the place it picked become deck-ui's
//! `session::Panel`, the way pairing.rs makes the menu's `Pair`. The link is the core's (`reader_link`, called here and
//! nowhere else in the editor), its QR pairing.rs's encoder. What the panel offers to pick is the public library the reader
//! itself reads (shared/api library.ts): the toons' ids and a toon's listed chapters – the server lists only those
//! (Library.php `seasonOverview`), so nothing is filtered here. The editor does the HTTP; the addresses and the reading of
//! the answers are here.
use crate::linked::{reader_link, Readers};
use crate::relay::encode;
use deck_ui::session::{Heard, Panel, Place, Reader};
use serde_json::Value;

/// What the editor knows of its session: the viewer's base (deck-link `app_url`), the session's code, the relay client's
/// line, the place picked, what there is to pick, the readers.
pub struct Facts<'a> {
    pub base: &'a str,
    pub code: &'a str,
    pub state: &'a str,
    pub place: &'a Place,
    pub toons: &'a [String],
    pub chapters: &'a [u32],
    pub readers: &'a Readers,
}

/// The address a linked reader opens for `place`: the core's `reader_link`; empty without a session (no code) and until a
/// toon and a chapter are picked. The editor's one call of it (#530 changes its signature: one line here).
pub fn link(base: &str, place: &Place, code: &str) -> String {
    match (place.toon.trim(), place.chapter) {
        _ if code.trim().is_empty() => String::new(),
        ("", _) | (_, None) => String::new(),
        (toon, Some(n)) => reader_link(base, toon, n, Some(place.label.as_str()), code),
    }
}

/// The panel for `f`.
pub fn panel(f: &Facts) -> Panel {
    let link = link(f.base, f.place, f.code);
    let qr = if link.is_empty() { Default::default() } else { crate::pairing::qr(&link) };
    let r = f.readers;
    let readers = r.linked.iter().map(|(id, name)| Reader { id: id.clone(), name: name.clone(), drives: r.driver.as_deref() == Some(id.as_str()) }).collect();
    let last = r.last.as_ref().map(|h| Heard { kind: h.kind.clone(), characters: h.characters.clone(), from: h.from.clone() });
    Panel { code: f.code.to_string(), state: f.state.to_string(), place: f.place.clone(), toons: f.toons.to_vec(), chapters: f.chapters.to_vec(), link, qr, readers, last }
}

/// `GET <api>/library/toons` – the toons a reader sees (the public list, no draft)
pub fn toons_url(api: &str) -> String {
    format!("{}/library/toons", api.trim().trim_end_matches('/'))
}

/// `GET <api>/library/toons/<toon>` – a toon with its seasons and their listed chapters
pub fn toon_url(api: &str, toon: &str) -> String {
    format!("{}/library/toons/{}", api.trim().trim_end_matches('/'), encode(toon.trim()))
}

/// the toons' ids in the list's order (`GET /library/toons`: `[{ id, … }]`)
pub fn toons(json: &str) -> Result<Vec<String>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("the toons: {e}"))?;
    let list = v.as_array().ok_or("the toons: not a list")?;
    Ok(list.iter().filter_map(|t| t.get("id")?.as_str()).filter(|id| !id.is_empty()).map(str::to_string).collect())
}

/// a toon's chapter numbers, season by season as the library lists them (`GET /library/toons/<toon>`:
/// `{ seasons: [{ chapters: [{ number }] }] }`; a number may come as a string)
pub fn chapters(json: &str) -> Result<Vec<u32>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("the toon: {e}"))?;
    let seasons = v.get("seasons").and_then(Value::as_array).ok_or("the toon: no seasons")?;
    let number = |c: &Value| match c.get("number")? {
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    };
    Ok(seasons.iter().flat_map(|s| s.get("chapters").and_then(Value::as_array).into_iter().flatten()).filter_map(number).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::Presence;
    use serde_json::json;

    /// a session with two linked readers, Mira driving, and a story she sent
    fn readers() -> Readers {
        let mut r = Readers::default();
        let p: Presence = serde_json::from_value(json!({ "groups": {}, "ready": [], "readers": { "c4": { "nick": "Mira" }, "c5": {} }, "driver": "c4" })).unwrap();
        r.presence(&p);
        let join = json!({ "type": "joinScene", "params": { "characters": [{ "character": "catpope" }] } });
        r.heard("c4", join.as_object().unwrap());
        r
    }

    /// the QR's modules read back as a camera would: rqrr decodes the picture
    fn decode(qr: &deck_ui::menu::Qr) -> String {
        let quiet = 4;
        let side = qr.n + 2 * quiet;
        let mut img = rqrr::PreparedImage::prepare_from_greyscale(side * 4, side * 4, |x, y| {
            let (mx, my) = ((x / 4) as isize - quiet as isize, (y / 4) as isize - quiet as isize);
            let dark = mx >= 0 && my >= 0 && (mx as usize) < qr.n && (my as usize) < qr.n && qr.dark[my as usize * qr.n + mx as usize];
            if dark { 0 } else { 255 }
        });
        let grids = img.detect_grids();
        assert_eq!(grids.len(), 1, "one code in the picture");
        grids[0].decode().expect("the code decodes").1
    }

    #[test]
    fn the_panel_names_the_driving_reader_the_last_event_and_the_core_s_link() {
        let r = readers();
        let place = Place { toon: "catpope".into(), chapter: Some(3), label: "dialog-start".into() };
        let (toons, chapters) = (vec!["catpope".to_string()], vec![0, 1, 2, 3]);
        let f = Facts { base: "https://scutoons.com", code: "A7K2", state: "connected as c1", place: &place, toons: &toons, chapters: &chapters, readers: &r };
        let p = panel(&f);
        assert_eq!(p.driver().as_deref(), Some("Mira"), "the driving reader by name");
        assert_eq!(p.readers_line(), "2 linked readers · Mira drives");
        assert_eq!(p.last.as_ref().map(|h| (h.kind.as_str(), h.characters.clone(), h.from.as_str())), Some(("joinScene", vec!["character:catpope".to_string()], "c4")));
        assert_eq!(p.last_line(), "last: joinScene · character:catpope · from Mira");
        assert_eq!(p.link, reader_link("https://scutoons.com", "catpope", 3, Some("dialog-start"), "A7K2"), "the link is the core's");
        assert_eq!(p.link, "https://scutoons.com/catpope/chapter/3?label=dialog-start&live=A7K2");
        assert_eq!((p.code.as_str(), p.state.as_str(), p.chapters.len()), ("A7K2", "connected as c1", 4));
        assert_eq!(decode(&p.qr), p.link, "the QR decodes to the link");
    }

    #[test]
    fn no_link_and_no_qr_until_a_toon_and_a_chapter_are_picked() {
        let r = Readers::default();
        for place in [Place::default(), Place { toon: "catpope".into(), chapter: None, label: "x".into() }, Place { toon: " ".into(), chapter: Some(1), label: String::new() }] {
            let p = panel(&Facts { base: "https://scutoons.com", code: "A7K2", state: "", place: &place, toons: &[], chapters: &[], readers: &r });
            assert_eq!((p.link.as_str(), p.qr.n), ("", 0), "{place:?}");
        }
        let start = Place { toon: "catpope".into(), chapter: Some(0), label: "  ".into() };
        assert_eq!(link("https://scutoons.com", &start, " "), "", "no session, no link");
        assert_eq!(link("http://192.168.1.116:4100/", &start, "A7K2"), "http://192.168.1.116:4100/catpope/chapter/0?live=A7K2", "an empty label: the chapter's start");
    }

    #[test]
    fn the_library_s_toons_and_listed_chapters() {
        assert_eq!(toons_url("https://api.scutoons.com/api/"), "https://api.scutoons.com/api/library/toons");
        assert_eq!(toon_url("http://127.0.0.1:9000/api", "cat pope"), "http://127.0.0.1:9000/api/library/toons/cat%20pope");
        assert_eq!(toons(r#"[{ "id": "catpope", "uri": "/catpope" }, { "id": "" }, { "uri": "x" }, { "id": "feilchen" }]"#).unwrap(), ["catpope", "feilchen"]);
        let toon = r#"{ "entry": {}, "seasons": [{ "number": 1, "chapters": [{ "number": 0 }, { "number": "1" }, { "number": 2, "status": "coming-soon" }] }, { "number": 2, "chapters": [{ "number": 3 }, { "title": "no number" }] }] }"#;
        assert_eq!(chapters(toon).unwrap(), [0, 1, 2, 3], "every chapter the library lists, season by season");
        assert!(toons("{}").is_err() && chapters("[]").is_err() && chapters("not json").is_err());
    }
}
