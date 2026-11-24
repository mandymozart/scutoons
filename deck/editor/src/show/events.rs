//! What fires a row, in the Show workspace (lap 2 P7): the row's ⚡ slot in its head opens Find under it – its own ground,
//! a field, the scope pills This toon · Show · Universe, the lines ticked where the row fires on them. A press turns a line's
//! event on or off (a row fires on several), "manual" clears them, Enter takes the first found. The toon's story events
//! come from `GET /api/creator/story-events` (#521); where there is none (no toon, the token lacks the creator role,
//! offline) Find says why and a typed id is the event. The lines and the words are deck-show's inlet.rs, every edit goes
//! through `rows::edit` (checked there, refused in its words); this file does the HTTP (api.rs), lays Find out and hands
//! the presses in (RULES A1, A2). How and hold stay as they are.
use super::roles::Typing;
use super::{Hit, Host, Sheet};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::cards::fit;
use deck_kit::editor::controls::{self, Rect};
use deck_show::inlet::{self, Line, Scope, Source, SCOPES};
use deck_show::rows::Edit;
use deck_show::story;
use std::path::Path;

/// the most lines Find shows; the rest are counted
const MOST: usize = 12;

impl Host {
    /// the toon whose story events are listed: `--toon`, else the book's
    pub(crate) fn story_toon(&self) -> Option<String> {
        self.toon.clone().or_else(|| self.book.as_ref().map(|b| b.toon().to_string())).filter(|t| !t.is_empty())
    }

    /// whose story events are listed first: `--toon`, else the book's toons, the home first (#708)
    fn story_toons(&self) -> Vec<String> {
        match (&self.toon, &self.book) {
            (Some(t), _) if !t.is_empty() => vec![t.clone()],
            (_, Some(b)) => b.toons().into_iter().filter(|t| !t.is_empty()).map(str::to_string).collect(),
            _ => Vec::new(),
        }
    }

    /// the same, for the problems list (mod.rs)
    pub(super) fn toon_now(&self) -> Option<String> {
        self.story_toon()
    }

    /// the story events and questions (#550: one answer), asked once – every toon's, the show's first – or why there are none
    pub(super) fn story_load(&mut self) {
        if self.story.is_none() {
            let toons = self.story_toons();
            self.story = Some(match toons.is_empty() {
                true => Err(story::no_toon()),
                false => match self.api.story_events() {
                    Ok(text) => story::read_all(&text, &toons),
                    Err((status, detail)) => Err(story::refused(status, &detail)),
                },
            });
        }
    }

    /// the slot's Find opened: the story events, asked once; its field open for typing
    pub(super) fn events_open(&mut self) {
        self.story_load();
        self.sheet = Some(Sheet::Event);
        self.flashing = None;
        self.event_query.clear();
        self.typing = Some((Typing::Event, Field::with("")));
    }

    pub(super) fn events_close(&mut self) {
        if self.sheet == Some(Sheet::Event) {
            self.sheet = None;
        }
        if matches!(self.typing, Some((Typing::Event, _))) {
            self.typing = None;
        }
    }

    /// a check run's list from a file (`--story-events FILE`, the API's answer) in place of the API's
    pub fn story_from_file(&mut self, file: &Path) {
        let toons = self.story_toons();
        self.story = Some(std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display())).and_then(|t| story::read_all(&t, &toons)));
    }

    pub(super) fn events_press(&mut self, hit: Hit) {
        match hit {
            Hit::Slot(i) => {
                let open = self.sheet == Some(Sheet::Event) && self.row == i;
                self.row = i;
                self.role = None;
                match open {
                    true => self.events_close(),
                    false => self.events_open(),
                }
            }
            Hit::EventPick(i) => {
                if let Some(l) = self.event_list().into_iter().nth(i).filter(|l| l.can) {
                    self.pick_event(l.event);
                }
            }
            Hit::EventType => self.typing = Some((Typing::Event, Field::with(&self.event_query))),
            Hit::EventScope(k) => {
                if let Some((s, _)) = SCOPES.get(k) {
                    self.event_scope = *s;
                }
            }
            _ => {}
        }
    }

    /// the inspected row's `event` on or off (none: none, by hand only); Find stays open for the next, "manual" closes it
    fn pick_event(&mut self, event: Option<String>) {
        let Some(row) = self.shown().rows.get(self.row).map(|r| r.id.clone()) else { return };
        let clears = event.is_none();
        if self.edit_rows(&Edit::Event { row, event }) && clears {
            self.events_close();
        }
    }

    /// what Find's field holds: the text being typed, else the query kept
    fn event_query_now(&self) -> String {
        match &self.typing {
            Some((Typing::Event, f)) => f.text.clone(),
            _ => self.event_query.clone(),
        }
    }

    /// Find's lines for the query and the scope (without a list: the show's own sources only)
    pub(super) fn event_list(&self) -> Vec<Line> {
        let Some(row) = self.shown().rows.get(self.row) else { return vec![] };
        let events = match &self.story {
            Some(Ok(s)) => s.events.as_slice(),
            _ => &[],
        };
        inlet::lines(self.shown(), &row.id, events, self.event_scope, &self.event_query_now())
    }

    /// Enter in Find's field: the first line found that can be picked (nothing typed: manual); none found and no story list:
    /// the typed text is the id
    pub(super) fn event_typed(&mut self, text: &str) {
        self.event_query = text.trim().to_string();
        match self.event_list().into_iter().find(|l| l.can) {
            Some(l) => self.pick_event(l.event),
            None if !matches!(self.story, Some(Ok(_))) && !self.event_query.is_empty() => self.pick_event(Some(self.event_query.clone())),
            None => self.status = story::NONE_FOUND.into(),
        }
        // Enter leaves the field; Find stays for the next pick until a press elsewhere or Escape
        if self.sheet == Some(Sheet::Event) {
            self.typing = Some((Typing::Event, Field::with("")));
            self.event_query.clear();
        }
    }
}

/// Find under the open slot, over everything (its ground takes the presses that miss its parts).
pub fn find(d: &mut DrawList, t: &Tokens, host: &mut Host, room: (f32, f32), pointer: (f32, f32)) {
    if host.sheet != Some(Sheet::Event) {
        return;
    }
    let show = host.shown().clone();
    let Some(row) = show.rows.get(host.row) else { return };
    let (pad, font, small, line_h) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
    let lines = host.event_list();
    let why = match &host.story {
        Some(Err(why)) => Some(why.clone()),
        _ => None,
    };
    let wide = host.event_scope != Scope::Toon;
    // the height: head, field, pills, why, the lines with their group heads, the rest counted
    let shown: Vec<(usize, &Line)> = lines.iter().enumerate().take(MOST).collect();
    let heads = if wide { shown.iter().filter(|(i, l)| *i == 0 || lines[i - 1].source != l.source).filter(|(_, l)| l.source != Source::Manual).count() } else { 0 };
    let w = t.size.rem(30.0).min(room.0 - 16.0);
    let h = pad + small * 1.6 + line_h + pad * 0.5 + line_h + pad * 0.5 + why.as_ref().map_or(0.0, |_| small * 1.6) + (shown.len() + heads) as f32 * line_h + if lines.len() > MOST || lines.is_empty() { line_h } else { 0.0 } + pad;
    let anchor = host.slot_at.unwrap_or(Rect { x: room.0 * 0.3, y: room.1 * 0.3, w: 0.0, h: 0.0 });
    let x = anchor.x.min(room.0 - w - 8.0).max(8.0);
    let below = anchor.y + anchor.h + 4.0;
    let y0 = if below + h > room.1 - 8.0 { (anchor.y - 4.0 - h).max(8.0) } else { below };
    let whole = Rect { x, y: y0, w, h };
    // its own ground: the stage under the panel's, so the band under it does not show through
    let round = t.size.radius * 2.0;
    d.round_rect(x, y0, w, h, round, t.color.stage);
    d.round_rect(x, y0, w, h, round, t.color.panel());
    d.outline(x, y0, w, h, (round, 1.0), t.color.line);
    host.hits.push((whole, Hit::EventGround));
    let (ix, iw) = (x + pad, w - 2.0 * pad);
    let mut y = y0 + pad;
    d.sans(ix, y, small, &fit(d, &inlet::title(&show, &row.id), small, iw), t.color.muted);
    y += small * 1.6;
    // the field: Find, what is typed
    let typing = host.typing.as_ref().filter(|(what, _)| *what == Typing::Event).map(|(_, f)| f.text.clone());
    let field = Rect { x: ix, y, w: iw, h: line_h - 2.0 };
    d.round_rect(field.x, field.y, field.w, field.h, field.h * 0.5, t.color.control);
    if typing.is_some() {
        d.outline(field.x, field.y, field.w, field.h, (field.h * 0.5, 1.0), t.color.text);
    }
    let icon = small * 1.1;
    d.icon(field.x + pad * 0.75, field.y + (field.h - icon) * 0.5, icon, "search", t.color.muted);
    let query = typing.clone().unwrap_or_else(|| host.event_query.clone());
    let (text, ink) = match (&typing, query.is_empty()) {
        (Some(q), _) => (format!("{q}|"), t.color.text),
        (None, true) => ("Find".to_string(), t.color.muted),
        (None, false) => (query.clone(), t.color.text),
    };
    d.sans(field.x + pad * 1.25 + icon, field.y + (field.h - font * 1.25) * 0.5, font, &fit(d, &text, font, field.w - icon - 2.0 * pad), ink);
    host.hits.push((field, Hit::EventType));
    y += line_h + pad * 0.5;
    // the scope pills
    let on = SCOPES.iter().position(|(s, _)| *s == host.event_scope).unwrap_or(0);
    let names: Vec<&str> = SCOPES.iter().map(|(_, n)| *n).collect();
    let ph = line_h - 4.0;
    for (k, r) in controls::pills_rounded(d, t, (ix, y, ph), &names, on, ph * 0.5).into_iter().enumerate() {
        host.hits.push((r, Hit::EventScope(k)));
    }
    y += line_h + pad * 0.5;
    if let Some(why) = &why {
        d.sans(ix, y, small, &fit(d, why, small, iw), t.color.warn);
        y += small * 1.6;
    }
    if lines.is_empty() {
        d.sans(ix, y + (line_h - font * 1.25) * 0.5, font, story::NONE_FOUND, t.color.muted);
    }
    let label_w = iw * 0.42;
    for (i, l) in shown {
        // wider than the toon: the lines grouped by where they come from
        if wide && l.source != Source::Manual && (i == 0 || lines[i - 1].source != l.source) {
            controls::label(d, t, ix, y + (line_h - small * 1.25) * 0.5, l.source.word(), t.color.muted);
            y += line_h;
        }
        let r = Rect { x: x + pad * 0.5, y, w: w - pad, h: line_h };
        if l.can && r.has(pointer.0, pointer.1) {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.hover);
        }
        if l.on {
            d.disc(ix + 3.0, y + line_h * 0.5, 3.0, t.color.text);
        }
        let ink = if l.can { t.color.text } else { t.color.muted };
        let ty = y + (line_h - font * 1.25) * 0.5;
        d.text(ix + pad, ty, font, &fit(d, &l.label, font, label_w - pad * 1.5), ink);
        d.sans(ix + label_w, ty + (font - small) * 0.5, small, &fit(d, &l.detail, small, iw - label_w), t.color.muted);
        if l.can {
            host.hits.push((r, Hit::EventPick(i)));
        }
        y += line_h;
    }
    if lines.len() > MOST {
        d.sans(ix, y + (line_h - small * 1.25) * 0.5, small, &format!("{} more – type to find", lines.len() - MOST), t.color.muted);
    }
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

    /// lap 2 P7's done-when, the editor's half: the row's ⚡ slot opens Find under it with the toon's story events; a press
    /// turns an event on or off (a row fires on several, Find stays), typing finds, the pills widen it to the show's rows,
    /// a press elsewhere closes it, "manual" clears the row
    #[test]
    fn the_slot_opens_find_and_a_row_takes_several_events() {
        let mut host = host(serve(200, EVENTS));
        host.press(Hit::Row(1));
        press(&mut host, |h| *h == Hit::Slot(1));
        assert_eq!(host.sheet, Some(Sheet::Event));
        assert!(host.slot_at.is_some(), "Find hangs under the slot");
        let labels: Vec<String> = host.event_list().iter().map(|e| e.label.clone()).collect();
        assert_eq!(labels, ["manual", "ch3.dialog-start", "no id", "ch3.yellowmouse-enters", "ch3.vigil", "ch3.end"]);
        // the line without an id is drawn but is no place to press
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        host.hits.clear();
        author::draw(&mut d, &t, &mut host, 1600.0, 900.0, (0.0, 0.0));
        let picks: Vec<Hit> = host.hits.iter().map(|(_, h)| *h).filter(|h| matches!(h, Hit::EventPick(_))).collect();
        assert_eq!(picks, [Hit::EventPick(0), Hit::EventPick(1), Hit::EventPick(3), Hit::EventPick(4), Hit::EventPick(5)]);
        // a second event: both fire it, Find stays open for the next
        press(&mut host, |h| *h == Hit::EventPick(4));
        assert_eq!(host.status, "row 2 dialog fires on ch3.dialog-start, ch3.vigil");
        assert_eq!(host.sheet, Some(Sheet::Event));
        assert_eq!(deck_show::view::inspector(host.shown(), "dialog")[0].1[0], ("fires on".to_string(), "event: ch3.dialog-start, ch3.vigil".to_string()));
        assert_eq!(host.shown().rows[1].fire.how, deck_show::How::Auto, "how stays");
        // pressed again: off; typed to find, Enter takes the first found – another row's event too
        press(&mut host, |h| *h == Hit::EventPick(1));
        assert_eq!(host.status, "row 2 dialog fires on ch3.vigil");
        typed(&mut host, "mouse");
        assert_eq!(host.status, "row 2 dialog fires on ch3.vigil, ch3.yellowmouse-enters");
        // Show: another row's fire
        press(&mut host, |h| *h == Hit::EventScope(1));
        let at = host.event_list().iter().position(|l| l.label == "row:opening").expect("the rows under Show");
        press(&mut host, |h| *h == Hit::EventPick(at));
        assert!(host.shown().rows[1].fire.event.contains("row:opening"));
        assert_eq!(host.shown().reads, Some(2), "stamped for the reader it needs");
        // a press elsewhere closes Find; manual clears the row and closes it
        host.press(Hit::Row(0));
        assert_eq!(host.sheet, None);
        press(&mut host, |h| *h == Hit::Slot(1));
        assert_eq!(host.row, 1);
        press(&mut host, |h| *h == Hit::EventPick(0));
        assert_eq!((host.status.as_str(), host.shown().rows[1].fire.event.is_empty(), host.sheet), ("row 2 dialog: manual", true, None));
    }

    /// the role live alone: 403 – Find says why and the typed id is the event
    #[test]
    fn without_the_creator_role_find_says_why_and_the_id_is_typed() {
        let mut host = host(serve(403, r#"{ "error": "forbidden", "role": "creator" }"#));
        press(&mut host, |h| *h == Hit::Slot(0));
        assert!(matches!(&host.story, Some(Err(why)) if why.contains("creator")), "{:?}", host.story);
        assert_eq!(host.event_list().iter().map(|l| l.label.as_str()).collect::<Vec<_>>(), ["manual"]);
        typed(&mut host, "ch3.start");
        assert_eq!(host.status, "row 1 opening fires on ch3.start");
    }
}
