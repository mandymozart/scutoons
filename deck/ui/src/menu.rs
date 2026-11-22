//! The deck's own menu (#386, draft 1 accepted 2026-11-11: "Design draft is accepted"; its four points: "1 through 4
//! makes sense"): the arcade splash, the main menu, its pages and the way back – one model for the native deck and the
//! browser deck. Plain data in (`Facts`: what the deck knows now), the kit's `Input` in, what to do out (`Answer`); the
//! page is drawn with the kit's arcade controls (`menu_draw.rs`). The adapters only map their events and show what this
//! answers: Live, Scenes and Settings are the deck's own pages (F6, F5, F4 on the native deck) – the menu opens them and
//! takes them back, it does not draw them. Avatar, Equip and Mappings come later: their pages open empty.
//!
//! The splash waits for START only when the deck is in no session; a deck told its outlet passes by itself once loaded
//! (point 1). In the browser's full screen Esc is the browser's: M, the status line or Start open the menu (point 2).
//!
//! the native deck starts on the start menu over the idle scene instead (Play, Join a show, Settings, Open creator)
//! and follows its stand (start.rs): a conductor drives it – the start pages give way; its conductor gone – "waiting for
//! <show>" with Leave. A show it plays as its own conductor: Esc opens its page, GO fires the next row, LEAVE stops it and
//! goes back to the start menu.
pub use deck_kit::base::input::Input;

/// the pages drawn with the kit and painted (`Menu::paint`)
pub mod draw;

/// A page of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// the logo and PRESS START, at the start of a deck in no session
    Splash,
    Main,
    /// the deck's own pages: F6, F5, F4
    Live,
    Scenes,
    Settings,
    /// the code and QR (the corner card that was)
    Pair,
    /// later: open empty
    Avatar,
    Equip,
    Mappings,
    /// the start menu over the idle scene, what Play lists, the wait for a conductor that is gone
    Start,
    Play,
    Waiting,
    /// the show this deck plays as its own conductor: GO, LEAVE
    Show,
    /// JOIN A SHOW: the session code (join.rs)
    Join,
}

impl Page {
    /// the deck's own page (F6, F5, F4): the adapter shows it where the deck has one
    pub fn own(self) -> bool {
        matches!(self, Page::Live | Page::Scenes | Page::Settings)
    }

    /// a page of the start, which gives way when a conductor drives the deck
    pub fn start(self) -> bool {
        matches!(self, Page::Start | Page::Play | Page::Waiting | Page::Show | Page::Join)
    }
}

/// A QR code's modules, row by row.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Qr {
    pub n: usize,
    pub dark: Vec<bool>,
}

/// Where the deck stands with its pairing (the corner card's states, #180, #470, #484).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Pair {
    /// nothing pairs here (no pairing service on this deck)
    #[default]
    Off,
    /// waiting for a member: the code as people type it, the link to type, its QR
    Waiting { code: String, hint: String, qr: Qr },
    /// paired, to whom (empty: not known here)
    Paired(String),
    /// a browser deck of a signed-in member is that member: no pairing (#484)
    SignedIn,
    /// it cannot pair now, why
    Offline(String),
}

impl Pair {
    /// the word on the menu's row and the splash's bottom line
    pub fn word(&self) -> &'static str {
        match self {
            Pair::Paired(_) => "PAIRED",
            Pair::SignedIn => "SIGNED IN",
            _ => "NOT PAIRED",
        }
    }
}

/// What the deck knows now, as plain values (the adapter reads its objects into it every frame).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Facts {
    /// the native deck: Quit, F4 / F5 / F6, Esc opens the menu (the browser: M)
    pub native: bool,
    /// the deck can open the browser on the creator (the Deck OS's session: OPEN CREATOR, the deck comes back after it)
    pub creator: bool,
    /// the deck's own pages it has: F6 (Live), F5 (Scenes), F4 (Settings)
    pub live_page: bool,
    pub scenes_page: bool,
    pub settings_page: bool,
    /// its session as one line (`OUTPUT C7 · TENOR: ANNA @ CHOIR`, `session_line`); none = in no session
    pub session: Option<String>,
    /// `role: name` in the session ("" = none)
    pub who: String,
    /// the scene playing (its name)
    pub scene: String,
    pub pair: Pair,
    /// the splash still waits for the chapter, the music
    pub loading: bool,
    /// screen pixels per CSS pixel (the native deck: its height / 1080); and the screen pixels the host covers along
    /// the top (the browser's status line)
    pub density: f32,
    pub top: f32,
    /// what Play lists (start.rs `play_list`)
    pub play: Vec<crate::start::Playable>,
    /// where Play's listing stands (an empty Play says why)
    pub listing: crate::start::Listing,
    /// the show this deck plays as its own conductor (none: none)
    pub showing: Option<crate::start::Showing>,
}

/// The session's line of the menu for the Live page's session: none = not in it.
pub fn session_line(outlet: &str, s: &crate::live::Session) -> Option<String> {
    let who = crate::live::who(s);
    s.joined.then(|| match outlet {
        "" => format!("{who} @ {}", s.code),
        o => format!("output {o} · {who} @ {}", s.code),
    })
}

/// What an input was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// not the menu's: the adapter passes it on (to its own open page, the stage)
    No,
    /// the menu took it
    Yes,
    /// START on the splash: it passes
    Started,
    /// QUIT (the native deck)
    Quit,
    /// OPEN CREATOR: the browser on the creator; closing it brings the deck back
    Creator,
    /// Play's row: load `Facts::play[n]`
    Play(usize),
    /// LEAVE: out of the session the deck waits in, or out of the show it plays on its own
    Leave,
    /// GO: the show this deck plays on its own fires its next row
    Go,
    /// JOIN A SHOW's code: join that session on the relay
    Join(String),
}

/// One row of the main menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Continue,
    Open(Page),
    Creator,
    Quit,
    /// the start menu's PLAY, JOIN A SHOW; Play's n-th row; the wait's LEAVE
    Play,
    Join,
    Pick(usize),
    Leave,
    /// the show page's GO
    Go,
}

/// The main menu's rows (OPEN CREATOR where the deck can open it, QUIT only on the native deck).
pub fn rows(f: &Facts) -> Vec<Row> {
    let mut out = vec![Row::Continue];
    out.extend([Page::Live, Page::Scenes, Page::Avatar, Page::Equip, Page::Mappings, Page::Settings, Page::Pair].map(Row::Open));
    if f.creator {
        out.push(Row::Creator);
    }
    if f.native {
        out.push(Row::Quit);
    }
    out
}

/// the start menu's rows: PLAY, JOIN A SHOW, SETTINGS, OPEN CREATOR where the deck can open it, QUIT on the native deck
pub fn start_rows(f: &Facts) -> Vec<Row> {
    let mut out = vec![Row::Play, Row::Join, Row::Open(Page::Settings)];
    if f.creator {
        out.push(Row::Creator);
    }
    if f.native {
        out.push(Row::Quit);
    }
    out
}

/// The rows of a list page (none: the page has none).
pub fn rows_of(page: Page, f: &Facts) -> Vec<Row> {
    match page {
        Page::Main => rows(f),
        Page::Start => start_rows(f),
        Page::Play => (0..f.play.len()).map(Row::Pick).collect(),
        Page::Waiting => vec![Row::Leave],
        Page::Show => vec![Row::Go, Row::Leave],
        _ => vec![],
    }
}

/// whether the deck has its own page for `page`
fn has(f: &Facts, page: Page) -> bool {
    match page {
        Page::Live => f.live_page,
        Page::Scenes => f.scenes_page,
        Page::Settings => f.settings_page,
        _ => false,
    }
}

/// A row's label, the value at its right, whether it is dimmed (nothing behind it yet).
pub fn words(row: Row, f: &Facts) -> (&'static str, String, bool) {
    let later = |label| (label, "LATER".to_string(), true);
    match row {
        Row::Continue => ("CONTINUE", String::new(), false),
        Row::Creator => ("OPEN CREATOR", String::new(), false),
        Row::Quit => ("QUIT", String::new(), false),
        Row::Open(Page::Live) if f.session.is_none() => ("LIVE", "NO SESSION".into(), !f.live_page),
        Row::Open(Page::Live) => ("LIVE", f.who.to_uppercase(), false),
        Row::Open(Page::Scenes) if !f.scenes_page => later("SCENES"),
        Row::Open(Page::Scenes) => ("SCENES", f.scene.to_uppercase(), false),
        Row::Open(Page::Settings) if !f.settings_page => later("SETTINGS"),
        Row::Open(Page::Settings) => ("SETTINGS", String::new(), false),
        Row::Open(Page::Pair) => ("PAIR THIS DECK", f.pair.word().into(), false),
        Row::Open(Page::Avatar) => later("AVATAR"),
        Row::Open(Page::Equip) => later("EQUIP"),
        Row::Open(Page::Mappings) => later("MAPPINGS"),
        Row::Open(_) => ("", String::new(), true),
        Row::Play if f.play.is_empty() => ("PLAY", f.listing.empty_word().into(), false),
        Row::Play => ("PLAY", crate::start::play_count(&f.play), false),
        Row::Join => ("JOIN A SHOW", f.session.as_ref().map_or("NO SESSION".into(), |_| f.who.to_uppercase()), false),
        Row::Pick(i) => match f.play.get(i) {
            Some(p) => ("", [p.demo.then_some("DEMO"), p.show.then_some("SHOW")].into_iter().flatten().collect::<Vec<_>>().join(" "), false),
            None => ("", String::new(), true),
        },
        Row::Leave => ("LEAVE", String::new(), false),
        Row::Go => ("GO", f.showing.as_ref().map(crate::start::Showing::next).unwrap_or_default(), f.showing.as_ref().is_none_or(|s| s.row >= s.rows)),
    }
}

/// What a place on the drawn page is (`menu_draw.rs` lays them out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Row(usize),
    Back,
}

/// The menu: the pages open (the last on top; none = closed), the main menu's cursor, and what was drawn last (its
/// places for the pointer, `menu_draw.rs`).
#[derive(Debug, Clone, Default)]
pub struct Menu {
    stack: Vec<Page>,
    cursor: usize,
    /// the cursor of the page under Play, while Play is open; the show waited for; the stand followed last
    under: usize,
    waiting: String,
    join: crate::join::Join,
    stand: Option<crate::start::Stand>,
    pub(crate) places: Vec<(deck_kit::base::draw::Rect, Hit)>,
    /// device pixels per logical one, as drawn
    pub(crate) scale: f32,
    pub(crate) drawn: Option<draw::Drawn>,
}

impl Menu {
    /// A deck at its start: on the splash when it is in no session (it waits for START), else nothing open.
    pub fn new(session: bool) -> Menu {
        Menu { stack: if session { vec![] } else { vec![Page::Splash] }, ..Menu::default() }
    }

    /// the page on top; none = closed
    pub fn top(&self) -> Option<Page> {
        self.stack.last().copied()
    }

    pub fn open(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn on_splash(&self) -> bool {
        self.top() == Some(Page::Splash)
    }

    /// the main menu's row the keys are on
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The deck's own page to show now (Live, Scenes, Settings – where the deck has it); none = none of them.
    pub fn own(&self, f: &Facts) -> Option<Page> {
        self.top().filter(|p| p.own() && has(f, *p))
    }

    /// F4 / F5 / F6: their page directly, over the scene, as before the menu – or closed when it is on top.
    pub fn direct(&mut self, page: Page) {
        self.stack = if self.top() == Some(page) { vec![] } else { vec![page] };
    }

    /// the show the waiting page waits for
    pub fn waiting(&self) -> &str {
        &self.waiting
    }

    /// the menu follows the deck's stand, only when it changed: a conductor drives the deck (or it is in its
    /// session, or plays its named scene) – the start pages give way; its conductor is gone – the waiting page; the deck
    /// is independent – the start menu, unless the person at it has another page open.
    pub fn follow(&mut self, stand: &crate::start::Stand) {
        use crate::start::Stand;
        if self.stand.as_ref() == Some(stand) {
            return;
        }
        self.stand = Some(stand.clone());
        match stand {
            Stand::Driven | Stand::Joined | Stand::Playing => self.stack.retain(|p| !p.start()),
            Stand::Waiting(show) => {
                self.waiting = show.clone();
                (self.stack, self.cursor) = (vec![Page::Waiting], 0);
            }
            Stand::Menu => {
                if self.top().is_none_or(|p| p == Page::Waiting || p == Page::Splash) {
                    (self.stack, self.cursor) = (vec![Page::Start], 0);
                }
            }
        }
    }

    /// the code page's keys: its place under the pointer is what a move or a press is on
    fn join_input(&mut self, i: Input) -> Answer {
        use crate::join::Step;
        let on = match i {
            Input::Point(x, y) | Input::Press(x, y) => {
                let s = self.scale.max(1.0);
                self.places.iter().find(|(at, _)| at.has(x / s, y / s)).map(|(_, h)| *h)
            }
            _ => None,
        };
        match self.join.input(i, on) {
            Step::Stay => Answer::Yes,
            Step::Join(code) => Answer::Join(code),
            Step::Back { joining } => {
                self.back();
                if joining { Answer::Leave } else { Answer::Yes }
            }
        }
    }

    /// a key of a keyboard for the code page; whether the page took it (it is not on top: no)
    pub fn typed(&mut self, t: crate::join::Typed) -> bool {
        self.top() == Some(Page::Join) && self.join.typed(t)
    }

    /// the relay's answer while a join is under way (`now` in seconds): done = the page closes with the others of the
    /// start; a failure stays on the page, said
    pub fn settle(&mut self, now: f32, reply: &crate::join::Reply) -> crate::join::Outcome {
        let outcome = self.join.settle(now, reply);
        if matches!(outcome, crate::join::Outcome::In(_)) {
            self.stack.retain(|p| *p != Page::Join);
        }
        outcome
    }

    /// the join could not even start: the page says why and is typed in again
    pub fn join_failed(&mut self, why: String) {
        self.join.fail(why);
    }

    /// whether the code page waits for the relay
    pub fn joining(&self) -> bool {
        self.join.joining()
    }

    /// the stage was made anew: what shows is painted again with the next `paint`
    pub fn again(&mut self) {
        self.drawn = None;
    }

    /// everything closed: back to the scene
    pub fn close(&mut self) {
        self.stack.clear();
    }

    fn back(&mut self) {
        if self.stack.pop() == Some(Page::Play) {
            self.cursor = self.under;
        }
    }

    fn take(&mut self, row: Row) -> Answer {
        match row {
            Row::Continue => self.close(),
            Row::Quit => return Answer::Quit,
            Row::Creator => {
                self.close();
                return Answer::Creator;
            }
            Row::Open(page) => self.stack.push(page),
            Row::Play => {
                (self.under, self.cursor) = (self.cursor, 0);
                self.stack.push(Page::Play);
            }
            Row::Join => {
                self.join.open();
                self.stack.push(Page::Join);
            }
            Row::Pick(i) => {
                self.close();
                return Answer::Play(i);
            }
            Row::Leave => {
                // out of its own show: the start menu again (the stand did not change)
                if self.top() == Some(Page::Show) {
                    (self.stack, self.cursor) = (vec![Page::Start], 0);
                } else {
                    self.close();
                }
                return Answer::Leave;
            }
            Row::Go => {
                self.close();
                return Answer::Go;
            }
        }
        Answer::Yes
    }

    /// One input, with what the deck knows now.
    pub fn input(&mut self, i: Input, f: &Facts) -> Answer {
        let hit = |m: &Menu, x: f32, y: f32| {
            let s = m.scale.max(1.0);
            m.places.iter().find(|(at, _)| at.has(x / s, y / s)).map(|(_, h)| *h)
        };
        let Some(top) = self.top() else {
            return match i {
                Input::Open => {
                    let page = if f.showing.is_some() { Page::Show } else { Page::Main };
                    (self.stack, self.cursor) = (vec![page], 0);
                    Answer::Yes
                }
                _ => Answer::No,
            };
        };
        match (top, i) {
            (Page::Splash, Input::Enter | Input::Press(..) | Input::Open) => {
                self.back();
                Answer::Started
            }
            (Page::Splash, Input::Point(..)) => Answer::No,
            (Page::Splash, _) => Answer::Yes,
            (Page::Join, i) => self.join_input(i),
            // the deck's own page has its keys and its pointer; back and the menu's key are the menu's
            (p, Input::Back | Input::Secondary) if p.own() && has(f, p) => {
                self.back();
                Answer::Yes
            }
            (p, _) if p.own() && has(f, p) => Answer::No,
            (Page::Main, Input::Open) => {
                self.close();
                Answer::Yes
            }
            (_, Input::Back | Input::Secondary) => {
                self.back();
                Answer::Yes
            }
            (p, Input::Up) if !rows_of(p, f).is_empty() => {
                self.cursor = self.cursor.saturating_sub(1);
                Answer::Yes
            }
            (p, Input::Down) if !rows_of(p, f).is_empty() => {
                self.cursor = (self.cursor + 1).min(rows_of(p, f).len() - 1);
                Answer::Yes
            }
            (p, Input::Enter) if !rows_of(p, f).is_empty() => {
                let rows = rows_of(p, f);
                self.take(rows[self.cursor.min(rows.len() - 1)])
            }
            (_, Input::Point(x, y)) => {
                if let Some(Hit::Row(r)) = hit(self, x, y) {
                    self.cursor = r;
                }
                Answer::Yes
            }
            (_, Input::Press(x, y)) => match hit(self, x, y) {
                Some(Hit::Row(r)) => {
                    self.cursor = r;
                    rows_of(top, f).get(r).map_or(Answer::Yes, |row| self.take(*row))
                }
                Some(Hit::Back) => {
                    self.back();
                    Answer::Yes
                }
                _ => Answer::Yes,
            },
            // a page of its own (Pair, a later one, one this deck lacks): M closes the menu, the rest is nothing yet
            (_, Input::Open) => {
                self.close();
                Answer::Yes
            }
            _ => Answer::Yes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native() -> Facts {
        Facts { native: true, live_page: true, scenes_page: true, settings_page: true, density: 1.0, ..Facts::default() }
    }

    fn browser() -> Facts {
        Facts { live_page: true, density: 2.0, ..Facts::default() }
    }

    /// the main menu's row of `page`
    fn row_of(f: &Facts, page: Page) -> usize {
        rows(f).iter().position(|r| *r == Row::Open(page)).expect("a row")
    }

    fn down_to(m: &mut Menu, f: &Facts, row: usize) {
        for _ in 0..row {
            assert_eq!(m.input(Input::Down, f), Answer::Yes);
        }
    }

    #[test]
    fn the_splash_waits_for_start_only_outside_a_session() {
        let f = native();
        let mut m = Menu::new(false);
        assert!(m.on_splash());
        assert_eq!(m.input(Input::Down, &f), Answer::Yes, "the splash keeps the keys");
        assert_eq!(m.input(Input::Point(3.0, 3.0), &f), Answer::No, "a moving pointer is nobody's there");
        assert_eq!(m.input(Input::Enter, &f), Answer::Started);
        assert!(!m.open());
        let mut m = Menu::new(false);
        assert_eq!(m.input(Input::Press(10.0, 10.0), &f), Answer::Started, "a press anywhere is START");
        // a deck told its outlet: nothing waits for a key on stage
        assert!(!Menu::new(true).open());
    }

    #[test]
    fn esc_opens_the_menu_continue_closes_it_and_quit_is_the_native_decks() {
        let f = native();
        let mut m = Menu::new(true);
        assert_eq!(m.input(Input::Up, &f), Answer::No, "closed: the keys are the stage's");
        assert_eq!(m.input(Input::Open, &f), Answer::Yes);
        assert_eq!((m.top(), m.cursor()), (Some(Page::Main), 0));
        assert_eq!(m.input(Input::Enter, &f), Answer::Yes);
        assert!(!m.open(), "CONTINUE closes");
        m.input(Input::Open, &f);
        down_to(&mut m, &f, rows(&f).len() + 3);
        assert_eq!(m.cursor(), rows(&f).len() - 1, "the cursor stops at the last row");
        assert_eq!(m.input(Input::Enter, &f), Answer::Quit);
        assert!(!rows(&browser()).contains(&Row::Quit), "the browser deck has no quit");
        assert_eq!(rows(&browser()).len(), rows(&f).len() - 1);
    }

    #[test]
    fn open_creator_sits_above_quit_only_where_the_deck_can_open_it() {
        let f = Facts { creator: true, ..native() };
        assert!(!rows(&native()).contains(&Row::Creator), "a desktop deck has no creator row");
        let r = rows(&f);
        assert_eq!(r[r.len() - 2..], [Row::Creator, Row::Quit]);
        assert_eq!(words(Row::Creator, &f), ("OPEN CREATOR", String::new(), false));
        let mut m = Menu::new(true);
        m.input(Input::Open, &f);
        down_to(&mut m, &f, r.len() - 2);
        assert_eq!(m.input(Input::Enter, &f), Answer::Creator);
        assert!(!m.open(), "the menu closes behind it: the deck comes back on its scene");
    }

    #[test]
    fn back_goes_one_step_from_every_page() {
        let f = native();
        for page in [Page::Live, Page::Scenes, Page::Settings, Page::Pair, Page::Avatar, Page::Equip, Page::Mappings] {
            let mut m = Menu::new(true);
            m.input(Input::Open, &f);
            down_to(&mut m, &f, row_of(&f, page));
            assert_eq!(m.input(Input::Enter, &f), Answer::Yes);
            assert_eq!(m.top(), Some(page));
            assert_eq!(m.input(Input::Back, &f), Answer::Yes, "{page:?}");
            assert_eq!(m.top(), Some(Page::Main), "{page:?}: back on the main menu");
            m.input(Input::Back, &f);
            assert!(!m.open(), "and from there to the scene");
        }
    }

    #[test]
    fn the_decks_own_pages_keep_their_keys_and_show_where_the_deck_has_them() {
        let f = native();
        let mut m = Menu::new(true);
        m.input(Input::Open, &f);
        down_to(&mut m, &f, row_of(&f, Page::Scenes));
        m.input(Input::Enter, &f);
        assert_eq!(m.own(&f), Some(Page::Scenes), "the F5 switcher shows");
        assert_eq!(m.input(Input::Down, &f), Answer::No, "its arrows are its own");
        assert_eq!(m.input(Input::Enter, &f), Answer::No);
        assert_eq!(m.input(Input::Secondary, &f), Answer::Yes, "a right click goes back");
        assert_eq!((m.top(), m.own(&f)), (Some(Page::Main), None));
        // the browser deck has no switcher: SCENES is a later row and opens empty, the menu keeps the keys
        let b = browser();
        assert_eq!(words(Row::Open(Page::Scenes), &b), ("SCENES", "LATER".into(), true));
        let mut m = Menu::new(true);
        m.input(Input::Open, &b);
        down_to(&mut m, &b, row_of(&b, Page::Scenes));
        m.input(Input::Enter, &b);
        assert_eq!((m.top(), m.own(&b)), (Some(Page::Scenes), None));
        assert_eq!(m.input(Input::Down, &b), Answer::Yes);
    }

    #[test]
    fn f4_f5_f6_still_open_their_page_directly() {
        let f = native();
        let mut m = Menu::new(true);
        m.direct(Page::Settings);
        assert_eq!(m.own(&f), Some(Page::Settings));
        m.direct(Page::Live);
        assert_eq!(m.own(&f), Some(Page::Live), "F6 over F4: the other page");
        m.direct(Page::Live);
        assert!(!m.open(), "F6 again closes it");
        m.direct(Page::Scenes);
        assert_eq!(m.input(Input::Back, &f), Answer::Yes);
        assert!(!m.open(), "Esc on a page opened directly goes back to the scene");
        // from the menu: F4 replaces it, Esc leaves to the scene
        m.input(Input::Open, &f);
        m.direct(Page::Settings);
        m.input(Input::Back, &f);
        assert!(!m.open());
    }

    #[test]
    fn a_later_page_opens_empty_and_m_closes_the_menu() {
        let f = browser();
        let mut m = Menu::new(true);
        m.input(Input::Open, &f);
        down_to(&mut m, &f, row_of(&f, Page::Equip));
        assert_eq!(words(Row::Open(Page::Equip), &f), ("EQUIP", "LATER".into(), true));
        m.input(Input::Enter, &f);
        assert_eq!((m.top(), m.own(&f)), (Some(Page::Equip), None));
        assert_eq!(m.input(Input::Enter, &f), Answer::Yes, "nothing to take there yet");
        assert_eq!(m.input(Input::Open, &f), Answer::Yes);
        assert!(!m.open(), "M closes the whole menu");
    }

    fn playable(value: &str, label: &str, demo: bool) -> crate::start::Playable {
        crate::start::Playable { value: value.into(), label: label.into(), demo, show: value.starts_with("show:") }
    }

    #[test]
    fn an_independent_deck_starts_on_the_start_menu_and_play_loads_a_scene() {
        use crate::start::Stand;
        let f = Facts { creator: true, play: vec![playable("scene:sandy-dunes", "Dunes", true), playable("scene:nave", "Nave", false)], ..native() };
        let mut m = Menu::default();
        m.follow(&Stand::Menu);
        assert_eq!(m.top(), Some(Page::Start));
        assert_eq!(start_rows(&f), [Row::Play, Row::Join, Row::Open(Page::Settings), Row::Creator, Row::Quit]);
        assert_eq!(words(Row::Play, &f), ("PLAY", "2 SCENES".into(), false));
        // an empty Play says why
        let empty = |listing| words(Row::Play, &Facts { listing, ..native() }).1;
        assert_eq!(
            [empty(crate::start::Listing::Loading), empty(crate::start::Listing::Offline), empty(crate::start::Listing::Answered)],
            ["LOADING", "OFFLINE", "NOTHING YET"]
        );
        assert_eq!(words(Row::Join, &f), ("JOIN A SHOW", "NO SESSION".into(), false));
        assert_eq!(m.input(Input::Down, &f), Answer::Yes);
        assert_eq!(m.input(Input::Enter, &f), Answer::Yes);
        assert_eq!((m.top(), m.own(&f)), (Some(Page::Join), None), "JOIN A SHOW opens the code page");
        m.input(Input::Back, &f);
        assert_eq!((m.top(), m.cursor()), (Some(Page::Start), 1));
        m.input(Input::Up, &f);
        m.input(Input::Enter, &f);
        assert_eq!(m.top(), Some(Page::Play));
        assert_eq!(words(Row::Pick(0), &f).1, "DEMO");
        m.input(Input::Back, &f);
        assert_eq!((m.top(), m.cursor()), (Some(Page::Start), 0), "back from Play: the start menu's cursor again");
        m.input(Input::Enter, &f);
        m.input(Input::Down, &f);
        assert_eq!(m.input(Input::Enter, &f), Answer::Play(1));
        assert!(!m.open(), "the scene loads, the menu closes");
        m.follow(&Stand::Menu);
        assert!(!m.open(), "the same stand again opens nothing");
    }

    #[test]
    fn a_show_picked_plays_esc_opens_its_page_go_fires_and_leave_goes_back_to_the_start_menu() {
        use crate::start::{Showing, Stand};
        let mut f = Facts { play: vec![playable("show:demo-new-scene", "demo-new-scene", true), playable("scene:nave", "Nave", false)], ..native() };
        assert_eq!(words(Row::Play, &f).1, "1 SCENES · 1 SHOWS");
        assert_eq!(words(Row::Pick(0), &f).1, "DEMO SHOW");
        let mut m = Menu::default();
        m.follow(&Stand::Menu);
        m.input(Input::Enter, &f);
        assert_eq!(m.input(Input::Enter, &f), Answer::Play(0));
        assert!(!m.open(), "the show plays, the menu closes");
        f.showing = Some(Showing { name: "demo-new-scene".into(), row: 1, rows: 3 });
        m.input(Input::Open, &f);
        assert_eq!(m.top(), Some(Page::Show), "Esc: the show's page, not the main menu");
        assert_eq!(rows_of(Page::Show, &f), [Row::Go, Row::Leave]);
        assert_eq!(words(Row::Go, &f), ("GO", "ROW 2".into(), false));
        assert_eq!(m.input(Input::Enter, &f), Answer::Go);
        assert!(!m.open(), "GO: back to the stage");
        m.input(Input::Open, &f);
        m.input(Input::Back, &f);
        assert!(!m.open(), "Esc again: the show plays on");
        f.showing = Some(Showing { name: "demo-new-scene".into(), row: 3, rows: 3 });
        assert!(words(Row::Go, &f).2, "after the last row GO is dimmed");
        m.input(Input::Open, &f);
        m.input(Input::Down, &f);
        assert_eq!(m.input(Input::Enter, &f), Answer::Leave);
        assert_eq!((m.top(), m.cursor()), (Some(Page::Start), 0), "LEAVE: the start menu again");
        m.follow(&Stand::Driven);
        assert!(!m.open(), "a conductor takes over from the show page too");
    }

    #[test]
    fn a_conductor_takes_over_and_the_start_menu_gives_way() {
        use crate::start::Stand;
        let f = native();
        let mut m = Menu::default();
        m.follow(&Stand::Menu);
        m.follow(&Stand::Driven);
        assert!(!m.open(), "driven: no start menu");
        m.follow(&Stand::Menu);
        assert_eq!(m.top(), Some(Page::Start), "independent again: the start menu");
        m.input(Input::Back, &f);
        m.input(Input::Open, &f);
        m.follow(&Stand::Driven);
        assert_eq!(m.top(), Some(Page::Main), "a page the person opened stays");
        m.follow(&Stand::Menu);
        assert_eq!(m.top(), Some(Page::Main));
    }

    #[test]
    fn a_rejoining_deck_waits_for_its_conductor_with_leave_in_reach() {
        use crate::start::Stand;
        let f = native();
        let mut m = Menu::default();
        m.follow(&Stand::Joined);
        assert!(!m.open(), "rejoining: no start menu");
        m.follow(&Stand::Waiting("CHOIR".into()));
        assert_eq!((m.top(), m.waiting()), (Some(Page::Waiting), "CHOIR"));
        assert_eq!(rows_of(Page::Waiting, &f), [Row::Leave]);
        assert_eq!(m.input(Input::Enter, &f), Answer::Leave);
        m.follow(&Stand::Menu);
        assert_eq!(m.top(), Some(Page::Start), "left: the start menu");
        m.follow(&Stand::Waiting("CHOIR".into()));
        m.follow(&Stand::Driven);
        assert!(!m.open(), "the conductor came back and drives it");
    }

    #[test]
    fn the_rows_say_the_session_the_scene_and_the_pairing() {
        let mut f = native();
        assert_eq!(words(Row::Open(Page::Live), &f), ("LIVE", "NO SESSION".into(), false));
        assert_eq!(words(Row::Open(Page::Pair), &f), ("PAIR THIS DECK", "NOT PAIRED".into(), false));
        let s = crate::live::Session { code: "CHOIR".into(), joined: true, role: Some("tenor".into()), name: "Anna".into(), ..Default::default() };
        f.session = session_line("C7", &s);
        f.who = crate::live::who(&s);
        f.scene = "fireflies".into();
        f.pair = Pair::SignedIn;
        assert_eq!(f.session.as_deref(), Some("output C7 · tenor: Anna @ CHOIR"));
        assert_eq!(words(Row::Open(Page::Live), &f), ("LIVE", "TENOR: ANNA".into(), false));
        assert_eq!(words(Row::Open(Page::Scenes), &f), ("SCENES", "FIREFLIES".into(), false));
        assert_eq!(words(Row::Open(Page::Pair), &f).1, "SIGNED IN");
        assert_eq!(session_line("", &crate::live::Session::default()), None);
    }

    /// JOIN A SHOW: a typed code joins, a refusal is said on the page, Esc returns to the start menu
    #[test]
    fn join_a_show_asks_for_a_code_and_says_what_the_relay_answered() {
        use crate::join::{Outcome, Reply, Typed};
        use crate::start::Stand;
        let f = native();
        let mut m = Menu::default();
        m.follow(&Stand::Menu);
        m.input(Input::Down, &f);
        m.input(Input::Enter, &f);
        assert_eq!(m.top(), Some(Page::Join));
        assert!(m.typed(Typed::Char('c')) && m.typed(Typed::Char('7')));
        assert_eq!(m.input(Input::Enter, &f), Answer::Join("C7".into()), "the typed code is asked for");
        assert!(m.joining());
        let refused = Reply { refused: Some("full: session full".into()), ..Reply::default() };
        assert_eq!(m.settle(1.0, &refused), Outcome::Failed("THE RELAY SAYS FULL: SESSION FULL".into()));
        assert_eq!(m.top(), Some(Page::Join), "the reason is on the page");
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("the code page");
        let said = |want: &str| layer.list.prims().iter().any(|p| matches!(*p, deck_kit::base::draw::Prim::Text { start, end, .. } if layer.list.text_of(start, end) == want));
        assert!(said("THE RELAY SAYS FULL: SESSION FULL") && said("CODE  C7_"));
        assert_eq!(m.input(Input::Back, &f), Answer::Yes);
        assert_eq!((m.top(), m.cursor()), (Some(Page::Start), 1), "Esc returns to the start menu");
        assert!(!m.typed(Typed::Char('x')), "typing off the page is nobody's");
    }

    #[test]
    fn the_welcome_closes_the_code_page_and_esc_while_joining_gives_it_up() {
        use crate::join::{Outcome, Reply, Typed};
        use crate::start::Stand;
        let f = native();
        let mut m = Menu::default();
        m.follow(&Stand::Menu);
        m.input(Input::Down, &f);
        m.input(Input::Enter, &f);
        m.typed(Typed::Char('a'));
        m.input(Input::Enter, &f);
        assert_eq!(m.input(Input::Back, &f), Answer::Leave, "the deck leaves what it began to join");
        assert_eq!(m.top(), Some(Page::Start));
        m.input(Input::Enter, &f);
        m.typed(Typed::Char('a'));
        m.input(Input::Enter, &f);
        assert_eq!(m.settle(2.0, &Reply { code: "A".into(), joined: true, ..Reply::default() }), Outcome::In("A".into()));
        assert_eq!(m.top(), Some(Page::Start), "joined: the page is gone, the stand takes the rest");
        m.follow(&Stand::Joined);
        assert!(!m.open());
    }
}
