//! The event a row fires on, in the Show workspace (#539; SHOW.md §7): the row inspector's "fires on" opens the event
//! picker in the stage's place – the same picker as the piece's (rows.rs `draw_picker`, one look) – listing the toon's
//! story events (`GET /api/creator/story-events`, #521), filtered as typed; a press, or Enter on the first found, sets the
//! row's event, "manual" clears it. Where there is no list (no toon, the token lacks the creator role, offline) the
//! picker says why and the typed id is the event. The address, the reading of the answer, the lines and the words are
//! deck-show's story.rs, every edit goes through `rows::edit` (checked there, refused in its words); this file does the
//! HTTP (api.rs), lays the picker out and hands the presses in (RULES A1, A2). How and hold stay as they are.
use super::roles::Typing;
use super::rows::{draw_picker, Line, Picker};
use super::{Hit, Host, Sheet};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::Rect;
use deck_show::rows::Edit;
use deck_show::story::{self, EventView};
use std::path::Path;

impl Host {
    /// the toon whose story events are listed: `--toon`, else the book's
    pub(crate) fn story_toon(&self) -> Option<String> {
        self.toon.clone().or_else(|| self.book.as_ref().map(|b| b.toon().to_string())).filter(|t| !t.is_empty())
    }

    /// the same, for the problems list (mod.rs)
    pub(super) fn toon_now(&self) -> Option<String> {
        self.story_toon()
    }

    /// the toon's story events and questions (#550: one answer), asked once – or why there are none
    pub(super) fn story_load(&mut self) {
        if self.story.is_none() {
            self.story = Some(match self.story_toon() {
                None => Err(story::no_toon()),
                Some(toon) => match self.api.story_events() {
                    Ok(text) => story::read(&text, &toon),
                    Err((status, detail)) => Err(story::refused(status, &detail)),
                },
            });
        }
    }

    /// the event picker opened: the toon's story events, asked once; its field open for typing
    pub(super) fn events_open(&mut self) {
        self.story_load();
        self.sheet = Some(Sheet::Event);
        self.flashing = None;
        self.event_query.clear();
        self.typing = Some((Typing::Event, Field::with("")));
    }

    /// a check run's list from a file (`--story-events FILE`, the API's answer) in place of the API's
    pub fn story_from_file(&mut self, file: &Path) {
        let toon = self.story_toon().unwrap_or_default();
        self.story = Some(std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display())).and_then(|t| story::read(&t, &toon)));
    }

    pub(super) fn events_press(&mut self, hit: Hit) {
        match hit {
            Hit::FiresOn => self.events_open(),
            Hit::EventPick(i) => {
                if let Some(e) = self.event_list().into_iter().nth(i).filter(|e| e.can) {
                    self.pick_event(e.event);
                }
            }
            Hit::EventType => self.typing = Some((Typing::Event, Field::with(&self.event_query))),
            _ => {}
        }
    }

    /// the inspected row fires on `event` (none: by hand only); done, the picker closes
    fn pick_event(&mut self, event: Option<String>) {
        let Some(row) = self.shown().rows.get(self.row).map(|r| r.id.clone()) else { return };
        if self.edit_rows(&Edit::Event { row, event }) {
            self.sheet = None;
            self.typing = None;
        }
    }

    /// what the picker's field holds: the text being typed, else the query kept
    fn event_query_now(&self) -> String {
        match &self.typing {
            Some((Typing::Event, f)) => f.text.clone(),
            _ => self.event_query.clone(),
        }
    }

    /// the picker's lines for the query (none without a list)
    pub(super) fn event_list(&self) -> Vec<EventView> {
        let Some(row) = self.shown().rows.get(self.row) else { return vec![] };
        match &self.story {
            Some(Ok(s)) => story::lines(self.shown(), &row.id, &s.events, &self.event_query_now()),
            _ => vec![],
        }
    }

    /// Enter in the picker's field: with a list, the first event found that can be picked (nothing typed: by hand only);
    /// without one, the typed text is the id (empty: by hand only)
    pub(super) fn event_typed(&mut self, text: &str) {
        self.event_query = text.trim().to_string();
        if !matches!(self.story, Some(Ok(_))) {
            let typed = Some(self.event_query.clone()).filter(|t| !t.is_empty());
            self.pick_event(typed);
            return;
        }
        match self.event_list().into_iter().find(|e| e.can) {
            Some(e) => self.pick_event(e.event),
            None => self.status = story::NONE_FOUND.into(),
        }
    }
}

/// The event picker in the stage's place.
pub fn picker(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let show = host.shown().clone();
    let Some(row) = show.rows.get(host.row) else {
        d.text(a.x + t.size.pad, a.y + t.size.pad, t.size.font, "Add a row first.", t.color.muted);
        return;
    };
    let title = story::picker_title(&show, &row.id, &host.story_toon().unwrap_or_default());
    let typing = host.typing.as_ref().filter(|(what, _)| *what == Typing::Event).map(|(_, f)| f.text.clone());
    let query = host.event_query_now();
    let views = host.event_list();
    let why = match &host.story {
        Some(Err(why)) => why.clone(),
        _ => String::new(),
    };
    let list = match &host.story {
        Some(Ok(_)) => Ok(views.iter().map(|v| Line { label: &v.label, detail: &v.detail, now: v.can && v.event == row.fire.event, can: v.can }).collect()),
        _ => Err((why.as_str(), story::TYPE_IT)),
    };
    let label = if matches!(host.story, Some(Ok(_))) { "find" } else { story::EVENT };
    let p = Picker { title: &title, field: (label, &query, typing.as_deref(), Hit::EventType), list, none: story::NONE_FOUND, pick: Hit::EventPick };
    draw_picker(d, t, host, a, p, pointer);
}

#[cfg(test)]
mod tests {
    use super::super::{api, author, Hit, Host, Sheet};
    use deck_kit::base::draw::DrawList;
    use deck_kit::base::tokens::Tokens;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use winit::keyboard::{Key, NamedKey};

    const TOP_TOY: &str = include_str!("../../../show/fixtures/top-toy.show.json");
    const EVENTS: &str = include_str!("../../../demos/story-events.json");

    /// an API that answers every request with `code` and `body`
    fn serve(code: u16, body: &'static str) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/api", l.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut h = String::new();
                while r.read_line(&mut h).is_ok_and(|n| n > 2) {
                    h.clear();
                }
                let _ = write!(s, "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        base
    }

    fn host(base: String) -> Host {
        // one folder per test: the tests of this file run on threads of one process and wrote the same file (a flaky run)
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("deck-editor-events-{}-{n}", std::process::id()));
        let file = dir.join("top-toy.show.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, TOP_TOY).unwrap();
        let mut h = Host::new(Some(file), None, None, base.clone());
        h.api = api::Api::with_cache(base, dir.join("shows"));
        h.toon = Some("catpope".into());
        h
    }

    /// one frame of the Show workspace, then a press on the first place that is `want`
    fn press(host: &mut Host, want: impl Fn(&Hit) -> bool) {
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, host, 1600.0, 900.0, (0.0, 0.0));
        let (r, hit) = host.hits.iter().rev().find(|(_, h)| want(h)).copied().unwrap_or_else(|| panic!("no such place: {:?}", host.hits.iter().map(|(_, h)| h).collect::<Vec<_>>()));
        assert_eq!(host.hit(r.x + r.w * 0.5, r.y + r.h * 0.5), Some(hit), "the place answers its press");
        host.press(hit);
    }

    fn typed(host: &mut Host, text: &str) {
        for c in text.chars() {
            host.type_key(&Key::Character(c.to_string().into()));
        }
        host.type_key(&Key::Named(NamedKey::Enter));
    }

    /// #539's done-when, the editor's half: the row inspector's "fires on" opens the picker with the toon's story events
    /// from the API; the row's event is picked by a press, found by typing, cleared by "manual" – never typed
    #[test]
    fn a_rows_event_is_picked_from_the_toons_story_events() {
        let mut host = host(serve(200, EVENTS));
        host.press(Hit::Row(1));
        press(&mut host, |h| *h == Hit::FiresOn);
        assert_eq!(host.sheet, Some(Sheet::Event));
        let labels: Vec<String> = host.event_list().iter().map(|e| e.label.clone()).collect();
        assert_eq!(labels, ["manual", "ch3.dialog-start", "no id", "ch3.yellowmouse-enters", "ch3.vigil", "ch3.end"]);
        // the line without an id is drawn but is no place to press
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, &mut host, 1600.0, 900.0, (0.0, 0.0));
        let picks: Vec<Hit> = host.hits.iter().map(|(_, h)| *h).filter(|h| matches!(h, Hit::EventPick(_))).collect();
        assert_eq!(picks, [Hit::EventPick(0), Hit::EventPick(1), Hit::EventPick(3), Hit::EventPick(4), Hit::EventPick(5)]);
        // a line pressed: the row fires on it, the picker closes
        press(&mut host, |h| *h == Hit::EventPick(4));
        assert_eq!(host.status, "row 2 dialog fires on ch3.vigil");
        assert_eq!(host.sheet, None);
        assert_eq!(deck_show::view::inspector(host.shown(), "dialog")[0].1[0], ("fires on".to_string(), "event: ch3.vigil".to_string()));
        assert_eq!(host.shown().rows[1].fire.how, deck_show::How::Auto, "how stays");
        // typed to find, Enter picks the first found; another row's event is refused and the picker stays
        press(&mut host, |h| *h == Hit::FiresOn);
        typed(&mut host, "mouse");
        assert_eq!(host.status, "not changed: row 'yellowmouse-enters': the event 'ch3.yellowmouse-enters' already fires row 'dialog'");
        assert_eq!(host.sheet, Some(Sheet::Event));
        press(&mut host, |h| *h == Hit::EventType);
        for _ in 0.."mouse".len() {
            host.type_key(&Key::Named(NamedKey::Backspace));
        }
        typed(&mut host, "miau");
        assert_eq!(host.status, "row 2 dialog fires on ch3.dialog-start");
        // by hand only clears it
        press(&mut host, |h| *h == Hit::FiresOn);
        press(&mut host, |h| *h == Hit::EventPick(0));
        assert_eq!((host.status.as_str(), host.shown().rows[1].fire.event.as_ref()), ("row 2 dialog: manual", None));
    }
    /// the role live alone: 403 – the picker says why and the typed id is the event
    #[test]
    fn without_the_creator_role_the_picker_says_why_and_the_id_is_typed() {
        let mut host = host(serve(403, r#"{ "error": "forbidden", "role": "creator" }"#));
        host.press(Hit::Row(0));
        press(&mut host, |h| *h == Hit::FiresOn);
        assert!(matches!(&host.story, Some(Err(why)) if why.contains("creator")), "{:?}", host.story);
        assert!(host.event_list().is_empty());
        typed(&mut host, "ch3.start");
        assert_eq!(host.status, "row 1 opening fires on ch3.start");
    }
}
