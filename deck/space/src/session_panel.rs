//! The deck editor's Session panel as plain data (#531; agents/research/SCENE-EVENTS.md §6.7): the conductor's view of the
//! linked readers (linked.rs `Readers`, relay.rs `Client::readers`) and the place it picked become deck-ui's
//! `session::Panel`, the way pairing.rs makes the menu's `Pair`. The link is the core's (`reader_link`, called here and
//! nowhere else in the editor), its QR pairing.rs's encoder. What the panel offers to pick is the public library the reader
//! itself reads (shared/api library.ts): the toons' ids and a toon's listed chapters – the server lists only those
//! (Library.php `seasonOverview`), so nothing is filtered here. The editor does the HTTP; the addresses and the reading of
//! the answers are here.
use crate::linked::{reader_link, Readers};
use crate::live_cameras::{camera_link, cameras_link};
use crate::relay::encode;
use deck_ui::session::{Camera, Heard, Panel, Place, Reader};
use serde_json::Value;

/// What the editor knows of its session: the viewer's base (deck-link `app_url`), the session's code, the relay client's
/// line, the place picked, what there is to pick, the readers.
pub struct Facts<'a> {
    pub base: &'a str,
    pub code: &'a str,
    /// the relay the conductor joined (#530; empty: none)
    pub relay: &'a str,
    pub state: &'a str,
    pub place: &'a Place,
    pub toons: &'a [String],
    pub chapters: &'a [u32],
    /// the picked chapter's jump places (`labels`)
    pub labels: &'a [String],
    pub readers: &'a Readers,
    /// the session's live cameras as the relay client last heard them
    pub cameras: &'a [protocol::LiveCamera],
    /// the camera picked in the list (its id): its own QR in place of the list's
    pub picked: Option<&'a str>,
}

/// The address a linked reader opens for `place`: the core's `reader_link`; empty without a session (no code) and until a
/// toon and a chapter are picked. The editor's one call of it (#530 changes its signature: one line here).
pub fn link(base: &str, place: &Place, code: &str, relay: &str) -> String {
    match (place.toon.trim(), place.chapter) {
        _ if code.trim().is_empty() => String::new(),
        ("", _) | (_, None) => String::new(),
        (toon, Some(n)) => reader_link(base, toon, n, Some(place.label.as_str()), code, Some(relay)),
    }
}

/// The panel for `f`.
pub fn panel(f: &Facts) -> Panel {
    let link = link(f.base, f.place, f.code, f.relay);
    let qr = if link.is_empty() { Default::default() } else { crate::pairing::qr(&link) };
    let r = f.readers;
    let readers = r.linked.iter().map(|(id, name)| Reader { id: id.clone(), name: name.clone(), drives: r.driver.as_deref() == Some(id.as_str()) }).collect();
    let last = r.last.as_ref().map(|h| Heard { kind: h.kind.clone(), characters: h.characters.clone(), from: h.from.clone() });
    let cameras = f.cameras.iter().map(|c| Camera { id: c.id.clone(), name: c.label.clone().filter(|l| !l.trim().is_empty()).unwrap_or_else(|| c.id.clone()), holder: c.nick.clone().or(c.holder.clone()), away: c.away == Some(true) }).collect();
    let picked = f.picked.and_then(|id| f.cameras.iter().position(|c| c.id == id));
    let cameras_link = match picked {
        _ if f.code.trim().is_empty() => String::new(),
        Some(i) => camera_link(f.base, &f.cameras[i].id, f.code, Some(f.relay)),
        None => cameras_link(f.base, f.code, Some(f.relay)),
    };
    let cameras_qr = if cameras_link.is_empty() { Default::default() } else { crate::pairing::qr(&cameras_link) };
    Panel { code: f.code.to_string(), state: f.state.to_string(), place: f.place.clone(), toons: f.toons.to_vec(), chapters: f.chapters.to_vec(), labels: f.labels.to_vec(), link, qr, readers, last, cameras, cameras_link, cameras_qr, picked }
}

/// `GET <api>/library/toons` – the toons a reader sees (the public list, no draft)
pub fn toons_url(api: &str) -> String {
    format!("{}/library/toons", api.trim().trim_end_matches('/'))
}

/// `GET <api>/library/toons/<toon>` – a toon with its seasons and their listed chapters
pub fn toon_url(api: &str, toon: &str) -> String {
    format!("{}/library/toons/{}", api.trim().trim_end_matches('/'), encode(toon.trim()))
}

/// `GET <api>/library/toons/<toon>/chapters/<n>` – a chapter with its groups
pub fn chapter_url(api: &str, toon: &str, chapter: u32) -> String {
    format!("{}/chapters/{chapter}", toon_url(api, toon))
}

/// a chapter's jump places (deck-core `places`: its groups' labels, in order, each once)
pub fn labels(json: &str) -> Result<Vec<String>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("the chapter: {e}"))?;
    Ok(deck_core::places::chapter_labels(&v))
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
        let f = Facts { base: "https://scutoons.com", code: "A7K2", relay: "", state: "connected as c1", place: &place, toons: &toons, chapters: &chapters, labels: &[], readers: &r, cameras: &[], picked: None };
        let p = panel(&f);
        assert_eq!(p.driver().as_deref(), Some("Mira"), "the driving reader by name");
        assert_eq!(p.readers_line(), "2 linked readers · Mira drives");
        assert_eq!(p.last.as_ref().map(|h| (h.kind.as_str(), h.characters.clone(), h.from.as_str())), Some(("joinScene", vec!["character:catpope".to_string()], "c4")));
        assert_eq!(p.last_line(), "last: joinScene · character:catpope · from Mira");
        assert_eq!(p.link, reader_link("https://scutoons.com", "catpope", 3, Some("dialog-start"), "A7K2", None), "the link is the core's");
        assert_eq!(p.link, "https://scutoons.com/catpope/chapter/3?label=dialog-start&live=A7K2");
        assert_eq!((p.code.as_str(), p.state.as_str(), p.chapters.len()), ("A7K2", "connected as c1", 4));
        assert_eq!(decode(&p.qr), p.link, "the QR decodes to the link");
    }

    #[test]
    fn no_link_and_no_qr_until_a_toon_and_a_chapter_are_picked() {
        let r = Readers::default();
        for place in [Place::default(), Place { toon: "catpope".into(), chapter: None, label: "x".into() }, Place { toon: " ".into(), chapter: Some(1), label: String::new() }] {
            let p = panel(&Facts { base: "https://scutoons.com", code: "A7K2", relay: "", state: "", place: &place, toons: &[], chapters: &[], labels: &[], readers: &r, cameras: &[], picked: None });
            assert_eq!((p.link.as_str(), p.qr.n), ("", 0), "{place:?}");
        }
        let start = Place { toon: "catpope".into(), chapter: Some(0), label: "  ".into() };
        assert_eq!(link("https://scutoons.com", &start, " ", ""), "", "no session, no link");
        assert_eq!(link("http://192.168.1.116:4100/", &start, "A7K2", "ws://192.168.1.116:7070/live/A7K2"), "http://192.168.1.116:4100/catpope/chapter/0?live=A7K2&relay=ws%3A%2F%2F192.168.1.116%3A7070", "an empty label: the chapter's start");
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

    #[test]
    fn a_chapter_s_labels_are_the_panel_s_places_to_pick() {
        assert_eq!(chapter_url("https://api.scutoons.com/api/", "cat pope", 3), "https://api.scutoons.com/api/library/toons/cat%20pope/chapters/3");
        let chapter = r#"{ "number": 3, "groups": [{ "id": "dialog-start" }, {}, { "id": "harbour" }, { "id": "dialog-start" }] }"#;
        let found = labels(chapter).unwrap();
        assert_eq!(found, ["dialog-start", "harbour"], "in order, each once");
        assert!(labels("not json").is_err());
        let (place, r) = (Place { toon: "catpope".into(), chapter: Some(3), label: "harbour".into() }, Readers::default());
        let p = panel(&Facts { base: "https://scutoons.com", code: "", relay: "", state: "", place: &place, toons: &[], chapters: &[3], labels: &found, readers: &r, cameras: &[], picked: None });
        assert_eq!(p.labels, ["dialog-start", "harbour"], "the sheet lists the chapter's labels");
    }

    /// the session's live cameras: each with its name and who holds it, the list's link a phone opens and its QR
    #[test]
    fn the_cameras_and_the_list_s_qr() {
        let cams: Vec<protocol::LiveCamera> = serde_json::from_value(serde_json::json!([
            { "id": "scene:duet", "label": "Duet", "holder": "c4", "nick": "Mira" },
            { "id": "scene:duet > shot:close" },
            { "id": "scene:wide", "holder": "c7", "away": true }
        ]))
        .unwrap();
        let (place, r) = (Place::default(), Readers::default());
        let p = panel(&Facts { base: "https://scutoons.com", code: "A7K2", relay: "", state: "", place: &place, toons: &[], chapters: &[], labels: &[], readers: &r, cameras: &cams, picked: None });
        assert_eq!(p.cameras.iter().map(deck_ui::session::camera_line).collect::<Vec<_>>(), ["Duet · held by Mira", "scene:duet > shot:close · free", "scene:wide · c7 away"]);
        assert_eq!(p.cameras_link, "https://scutoons.com/live/cameras?live=A7K2");
        assert!(p.cameras_qr.n > 0);
        let one = panel(&Facts { base: "https://scutoons.com", code: "A7K2", relay: "", state: "", place: &place, toons: &[], chapters: &[], labels: &[], readers: &r, cameras: &cams, picked: Some("scene:duet > shot:close") });
        assert_eq!((one.picked, one.cameras_link.as_str()), (Some(1), "https://scutoons.com/live/cameras/scene%3Aduet%20%3E%20shot%3Aclose?live=A7K2"), "a camera picked: its own QR");
        let none = panel(&Facts { base: "https://scutoons.com", code: "", relay: "", state: "", place: &place, toons: &[], chapters: &[], labels: &[], readers: &r, cameras: &[], picked: None });
        assert!(none.cameras_link.is_empty() && none.cameras_qr.n == 0, "no session: no link");
    }
}
