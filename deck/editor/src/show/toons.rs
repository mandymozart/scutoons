//! A show's toons: the home picked at New show (its name, then its toon), kept with the document (deck-show's `toons`,
//! `Edit::Toon`); where the show names none the picker sits in place – the library panel, the Session sheet's toon chips.
//! Once it has one, the inspector's toons row takes more in or out (#708, `Edit::ToggleToon`: a show that mixes toons reads
//! all their books). The toons are the public library's list, asked once on a thread.
use super::roles::Typing;
use super::rows::field;
use super::{Hit, Host, Sheet};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::rows::Edit;
use deck_show::{ClockChoice, Subtree};
use deck_space::session_panel;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// The library's toons, the picker's typed query, the new show's name.
#[derive(Debug, Default)]
pub struct Toons {
    pub list: Vec<String>,
    pub why: Option<String>,
    rx: Option<Receiver<Result<Vec<String>, String>>>,
    asked: bool,
    pub query: String,
    pub name: String,
}

const ON_ITS_WAY: &str = "the library's toons are on their way";

impl Host {
    /// the toon of the show shown, none while it names none
    pub fn show_toon(&self) -> Option<&str> {
        Some(self.shown().home()).filter(|t| !t.is_empty())
    }

    /// the toons row's chips: the library's toons, then any the show names that the list lacks; whether the show uses it
    pub(super) fn toons_row(&self) -> Vec<(String, bool)> {
        let used = &self.shown().toons;
        let mut out: Vec<(String, bool)> = self.toons.list.iter().map(|t| (t.clone(), used.contains(t))).collect();
        out.extend(used.iter().filter(|t| !self.toons.list.contains(t)).map(|t| (t.clone(), true)));
        out
    }

    /// the library's toons, asked once
    pub(super) fn toons_ask(&mut self) {
        if self.toons.asked {
            return;
        }
        self.toons.asked = true;
        let (tx, rx) = channel();
        let url = session_panel::toons_url(&self.api.base);
        std::thread::spawn(move || {
            let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build();
            let text = agent.get(&url).call().map_err(|e| format!("{url}: {e}")).and_then(|r| r.into_string().map_err(|e| format!("{url}: {e}")));
            let _ = tx.send(text.and_then(|t| session_panel::toons(&t)));
        });
        self.toons.rx = Some(rx);
    }

    /// each frame: the list once it came; asked once Show is drawn (the toons row and the pickers list it)
    pub(super) fn toons_poll(&mut self) {
        self.toons_ask();
        if let Some(Ok(answer)) = self.toons.rx.as_ref().map(Receiver::try_recv) {
            self.toons.rx = None;
            match answer {
                Ok(list) => self.toons.list = list,
                Err(e) => self.toons.why = Some(format!("the toons: {e}")),
            }
        }
    }

    /// New show: its name typed first, the toons asked
    pub(super) fn new_show_open(&mut self) {
        self.sheet = Some(Sheet::NewShow);
        self.flashing = None;
        (self.toons.name, self.toons.query) = (String::new(), String::new());
        self.typing = Some((Typing::NewName, Field::with("")));
        self.toons_ask();
    }

    fn toon_query_now(&self) -> String {
        match &self.typing {
            Some((Typing::ToonFind, f)) => f.text.clone(),
            _ => self.toons.query.clone(),
        }
    }

    /// the toons the query finds
    pub(super) fn toons_found(&self) -> Vec<String> {
        deck_show::document::toons_found(&self.toons.list, &self.toon_query_now()).into_iter().cloned().collect()
    }

    pub(super) fn toons_press(&mut self, hit: Hit) {
        match hit {
            Hit::NewName => self.typing = Some((Typing::NewName, Field::with(&self.toons.name))),
            Hit::ToonFind => self.typing = Some((Typing::ToonFind, Field::with(&self.toons.query))),
            Hit::ToonPick(i) => {
                if let Some(t) = self.toons_found().get(i).cloned() {
                    self.pick_toon(&t);
                }
            }
            Hit::ShowToon(i) => {
                if let Some(t) = self.toons_row().get(i).map(|(t, _)| t.clone()) {
                    self.edit_rows(&Edit::ToggleToon { toon: t });
                }
            }
            _ => {}
        }
    }

    /// Enter in a field: the name moves on to the toon; the query picks the one toon it finds
    pub(super) fn toon_typed(&mut self, what: &Typing, text: &str) {
        match what {
            Typing::NewName => {
                self.toons.name = text.trim().to_string();
                self.typing = Some((Typing::ToonFind, Field::with(&self.toons.query)));
            }
            _ => {
                self.toons.query = text.trim().to_string();
                match self.toons_found().as_slice() {
                    [one] => {
                        let one = one.clone();
                        self.pick_toon(&one);
                    }
                    [] => self.status = deck_show::document::NO_TOON_FOUND.into(),
                    _ => self.status = "more than one toon found – pick one".into(),
                }
            }
        }
    }

    /// a toon picked: in New show the show is made with it, else it is the shown show's
    fn pick_toon(&mut self, toon: &str) {
        if self.sheet == Some(Sheet::NewShow) {
            return self.make_show(toon);
        }
        if self.edit_rows(&Edit::Toon { to: toon.to_string() }) {
            self.toons.query.clear();
        }
    }

    fn make_show(&mut self, toon: &str) {
        match deck_show::document::new_show(&self.toons.name, toon) {
            Ok(s) => {
                self.status = format!("new show {} · toon {}", s.id, s.home());
                self.sub = Subtree::new(s, ClockChoice::Parent);
                (self.path, self.row, self.role, self.sheet, self.typing) = (Vec::new(), 0, None, None, None);
                (self.cells, self.columns, self.group) = (Vec::new(), Vec::new(), None);
                (self.toons.name, self.toons.query) = (String::new(), String::new());
                self.session.place = deck_ui::session::Place::default();
            }
            Err(e) => self.refuse(e),
        }
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// The toon picker in `(x, y, w)`: the toons found as chips (a press picks one), or why there are none, then the find
/// field. The height it took.
pub fn picker(d: &mut DrawList, t: &Tokens, host: &mut Host, (x, y, w): (f32, f32, f32)) -> f32 {
    let (small, row, pad) = (t.size.small, t.size.tree_row, t.size.pad);
    let found = host.toons_found();
    let note = match (&host.toons.why, host.toons.list.is_empty(), found.is_empty()) {
        (Some(why), true, _) => Some((why.clone(), t.color.warn)),
        (None, true, _) => Some((ON_ITS_WAY.to_string(), t.color.muted)),
        (_, false, true) => Some((deck_show::document::NO_TOON_FOUND.to_string(), t.color.muted)),
        _ => None,
    };
    let mut used = match note {
        Some((text, ink)) => {
            d.text(x, y + 2.0, small, &fit(d, &text, small, w), ink);
            small * 1.8
        }
        None => {
            let words: Vec<&str> = found.iter().map(String::as_str).collect();
            let (rects, h) = controls::chips(d, t, (x, y, w), &words, &vec![false; words.len()]);
            host.hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, Hit::ToonPick(i))));
            h + pad * 0.5
        }
    };
    let typing = host.typing.as_ref().filter(|(what, _)| *what == Typing::ToonFind).map(|(_, f)| f.text.clone());
    let query = host.toon_query_now();
    let r = field(d, t, (x, y + used, w), "find", &query, typing.as_deref());
    host.hits.push((r, Hit::ToonFind));
    used += row;
    used
}

/// The show's toons in the inspector (#708): every toon as a chip, the ones it uses lit, the home marked; a press takes one in
/// or out (the core refuses the last). The height it took.
pub fn of_show(d: &mut DrawList, t: &Tokens, host: &mut Host, (x, y, w): (f32, f32, f32)) -> f32 {
    let (font, row, pad) = (t.size.font, t.size.tree_row, t.size.pad);
    let home = host.show_toon().unwrap_or_default().to_string();
    let chips = host.toons_row();
    d.text(x, y + (row - font * 1.25) * 0.5, font, "toons", t.color.muted);
    let words: Vec<String> = chips.iter().map(|(c, _)| if *c == home { format!("{c} · home") } else { c.clone() }).collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let lit: Vec<bool> = chips.iter().map(|(_, on)| *on).collect();
    let (rects, h) = controls::chips(d, t, (x + w * 0.42, y + 2.0, w * 0.58), &words, &lit);
    host.hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, Hit::ShowToon(i))));
    h.max(row) + pad * 0.5
}

/// Where the show names no toon: why, and the picker right there. The height it took.
pub fn in_place(d: &mut DrawList, t: &Tokens, host: &mut Host, (x, y, w): (f32, f32, f32)) -> f32 {
    let small = t.size.small;
    d.text(x, y, small, &fit(d, deck_show::document::PICK_TOON, small, w), t.color.warn);
    small * 1.8 + picker(d, t, host, (x, y + small * 1.8, w))
}

/// New show as a dialog's body in `a`: its name, then its toon found and picked – a toon picked makes the show.
pub fn new_sheet(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
    let col = t.size.rem(5.0);
    let mut y = a.y;
    let typing = host.typing.as_ref().filter(|(what, _)| *what == Typing::NewName).map(|(_, f)| f.text.clone());
    let name = host.toons.name.clone();
    let r = super::dialog::labelled(d, t, (a.x, y, a.w, col), ("name", ""), (&name, typing.as_deref()), pointer);
    host.hits.push((r, Hit::NewName));
    y += row + pad;
    controls::label(d, t, a.x, y, "toon", t.color.muted);
    y += t.size.label * 1.25 + pad * 0.5;
    let typing = host.typing.as_ref().filter(|(what, _)| *what == Typing::ToonFind).map(|(_, f)| f.text.clone());
    let query = host.toon_query_now();
    let r = super::dialog::input(d, t, Rect { x: a.x, y, w: a.w, h: row - 4.0 }, (&query, typing.as_deref()), ("Find", true), pointer);
    host.hits.push((r, Hit::ToonFind));
    y += row + pad * 0.3;
    let foot = a.y + a.h - row - pad;
    let note = match (&host.toons.why, host.toons.list.is_empty()) {
        (Some(why), true) => Some((why.clone(), t.color.warn)),
        (None, true) => Some((ON_ITS_WAY.to_string(), t.color.muted)),
        _ => None,
    };
    if let Some((text, ink)) = note {
        d.sans(a.x, y + 4.0, font, &controls::fit_sans(d, font, &text, a.w), ink);
    } else {
        let found = host.toons_found();
        if found.is_empty() {
            d.sans(a.x, y + 4.0, font, deck_show::document::NO_TOON_FOUND, t.color.muted);
        }
        let lines: Vec<super::dialog::Tick> = found.into_iter().map(|toon| super::dialog::Tick { label: toon, detail: String::new(), on: false, warns: false, indent: false, tick: false }).collect();
        for (i, r) in super::dialog::ticks(d, t, Rect { x: a.x, y, w: a.w, h: foot - y - pad }, &lines, pointer).into_iter().enumerate() {
            host.hits.push((r, Hit::ToonPick(i)));
        }
    }
    let hits = super::dialog::foot(d, t, Rect { x: a.x, y: foot, w: a.w, h: row }, &super::dialog::Foot { left: &[("Cancel", Hit::DialogClose)], right: &[], primary: None }, pointer);
    host.hits.extend(hits);
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{press, typed};
    use super::super::{Hit, Host, Sheet};

    /// one Show frame in a window as tall as the screenshots' (the scene list gets its room), then a press on `want`
    fn tall_press(host: &mut Host, want: Hit) {
        let (mut d, t) = (deck_kit::base::draw::DrawList::new(0.6), deck_kit::base::tokens::Tokens::default());
        host.hits.clear();
        super::super::author::draw(&mut d, &t, host, 2560.0, 1440.0, (0.0, 0.0));
        super::super::scenes::panel(&mut d, &t, host, &mut crate::library::Library::new(std::env::temp_dir().join("deck-editor-toons-library")), (0.0, 0.0));
        super::super::dialog::draw(&mut d, &t, host, (2560.0, 1440.0), (0.0, 0.0));
        let (r, hit) = host.hits.iter().rev().find(|(_, h)| *h == want).copied().unwrap_or_else(|| panic!("no such place: {:?}", host.hits));
        assert_eq!(host.hit(r.x + r.w * 0.5, r.y + r.h * 0.5), Some(hit));
        host.press(hit);
    }

    fn host(toons: &[&str]) -> Host {
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9/api".into());
        host.toons.list = toons.iter().map(|t| t.to_string()).collect();
        host
    }

    #[test]
    fn new_show_asks_its_name_then_its_toon() {
        let mut host = host(&["catpope", "feilchen"]);
        host.press(Hit::Sheet(Sheet::NewShow));
        assert_eq!(host.sheet, Some(Sheet::NewShow));
        host.typing = None;
        press(&mut host, |h| *h == Hit::ToonPick(1));
        assert_eq!((host.status.as_str(), host.sheet), ("a new show needs a name", Some(Sheet::NewShow)));
        press(&mut host, |h| *h == Hit::NewName);
        typed(&mut host, "Top Toy Opening");
        typed(&mut host, "feil");
        assert_eq!(host.status, "new show top-toy-opening · toon feilchen");
        assert_eq!((host.shown().id.as_str(), host.shown().home(), host.sheet), ("top-toy-opening", "feilchen", None));
        assert_eq!(host.show_toon(), Some("feilchen"), "the book follows it");
    }

    #[test]
    fn where_the_show_names_no_toon_the_picker_sits_in_place() {
        let mut host = host(&["catpope", "feilchen"]);
        assert_eq!(host.show_toon(), None);
        host.press(Hit::ToonFind);
        typed(&mut host, "e");
        assert_eq!((host.status.as_str(), host.show_toon()), ("more than one toon found – pick one", None));
        tall_press(&mut host, Hit::ToonPick(0));
        assert_eq!((host.status.as_str(), host.show_toon()), ("the show's toon: catpope", Some("catpope")));
        host.book = Some(std::sync::Arc::new(deck_space::rig::book::Book::parse(super::super::rows::tests::BOOK).unwrap()));
        let mut lib = crate::library::Library::new(std::env::temp_dir().join("deck-editor-toons-library"));
        super::super::scenes::tests::frame(&mut host, &mut lib);
        assert!(!host.hits.iter().any(|(_, h)| matches!(h, Hit::ToonPick(_) | Hit::ToonFind)), "with its toon: the library again");
        assert!(!lib.browser.hits.is_empty());
    }

    /// #708: once the show has its toon, the inspector's toons row takes a second in (the book asks both, the home first)
    /// and out again; the last one stays
    #[test]
    fn the_inspectors_toons_row_takes_toons_in_and_out() {
        let mut host = host(&["catpope", "yellowmouse"]);
        host.edit_rows(&deck_show::rows::Edit::Toon { to: "catpope".into() });
        press(&mut host, |h| *h == Hit::ShowToon(1));
        assert_eq!((host.status.as_str(), host.shown().toons.as_slice()), ("the show's toons: catpope (home), yellowmouse", &["catpope".to_string(), "yellowmouse".to_string()][..]));
        assert_eq!(crate::toon::wanted(None, false, None, &host.shown().toons), ["catpope", "yellowmouse"], "the book follows both");
        press(&mut host, |h| *h == Hit::ShowToon(0));
        assert_eq!(host.shown().toons, ["yellowmouse"]);
        press(&mut host, |h| *h == Hit::ShowToon(1));
        assert!(host.status.contains("keeps its one toon"), "{}", host.status);
        assert_eq!(host.toons_row(), [("catpope".to_string(), false), ("yellowmouse".to_string(), true)]);
    }

    #[test]
    fn the_session_sheets_toon_sets_a_show_without_one_and_starts_at_the_shows() {
        let mut host = host(&["catpope", "feilchen"]);
        host.sheet = Some(Sheet::Session);
        press(&mut host, |h| *h == Hit::SessionToon(1));
        assert_eq!((host.show_toon(), host.session.place.toon.as_str()), (Some("feilchen"), "feilchen"));
        press(&mut host, |h| *h == Hit::SessionToon(0));
        assert_eq!((host.show_toon(), host.session.place.toon.as_str()), (Some("feilchen"), "catpope"), "the reader's place only, once the show has one");
        host.session.place = Default::default();
        host.session_open();
        assert_eq!(host.session.place.toon, "feilchen");
    }
}
