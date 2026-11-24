//! The menu drawn (#386, draft 1): every page laid out top down with the kit's live components (deck-kit `live/hud.rs`,
//! the HUD's arcade look) and painted by the kit through egui – one layer for the native deck and the browser deck, a
//! full screen the stage lays over the scene (deck-space `Stage::set_menu`, #485). Sizes are logical pixels (a cell of the pixel face): the
//! HUD's scale makes 16 px glyphs on a 1080p screen, 40 columns; on a phone (narrow, upright) the rows are a finger's –
//! 20 high, 40 px at 16 px glyphs, a value under its label. What can be pressed is kept for the pointer (`places`).
use super::{rows_of, words, Facts, Hit, Menu, Page, Pair, Sheet, SCENES};
use deck_kit::base::draw::{DrawList, Rect};
pub use deck_kit::base::screen::Layer;
use deck_kit::base::tokens::Tokens;
use deck_kit::live::hud::{self as look, Item, GLYPH, ROW};

/// the main menu's text width on a wide screen: 40 columns
const COLUMNS: f32 = 40.0 * GLYPH;
const PAD: f32 = 4.0;
/// the splash logo's width per height (deck-space splash.rs: the logo SVG, 492 × 193)
const LOGO_ASPECT: f32 = 492.0 / 193.0;

/// Device pixels per logical one on a `sw × sh` screen at `density` screen pixels per CSS pixel: the HUD's at 1080p (2)
/// per CSS pixel – a phone's 16 CSS px glyphs –, less where the screen is too small for it, never under 1.
pub fn scale(sw: f32, sh: f32, density: f32) -> f32 {
    let want = (2.0 * density.max(0.5)).round();
    let fits = (sh / 140.0).floor().min((sw / 180.0).floor());
    want.min(fits).max(1.0)
}

/// a finger's rows: the screen (in logical pixels) is narrower than the 40 columns, or upright
pub fn finger(lw: f32, lh: f32) -> bool {
    lw < COLUMNS + 20.0 || lh > lw * 1.2
}

/// what was drawn: the pages, the cursor, the facts, the screen, the show waited for (`Menu::drawn`)
pub(crate) type Drawn = (Vec<Page>, usize, Facts, (f32, f32), String, crate::join::Join, bool);

/// the parts of a page's body that can be pressed, and where the body is
struct Body<'a> {
    d: &'a mut DrawList,
    t: &'a Tokens,
    x: f32,
    w: f32,
    finger: bool,
    places: Vec<(Rect, Hit)>,
}

impl Body<'_> {
    fn line(&mut self, y: f32, left: &str, right: &str, muted: bool) -> f32 {
        look::line(self.d, self.t, (self.x, y, self.w), left, right, muted)
    }

    fn rule(&mut self, y: f32) -> f32 {
        look::rule(self.d, self.t, (self.x, y, self.w))
    }

    /// the head of a page: its name, and on a wide screen what goes back
    fn head(&mut self, y: f32, title: &str, back: &str) -> f32 {
        let back = if self.finger { "" } else { back };
        self.line(y, title, back, false)
    }

    /// on a finger's screen the last line goes back
    fn foot_back(&mut self, y: f32, f: &Facts) -> f32 {
        let word = if f.native { "ESC BACK" } else { "BACK CLOSE" };
        self.places.push((Rect { x: self.x, y, w: self.w, h: ROW }, Hit::Back));
        self.line(y, word, "", true)
    }
}

fn title(page: Page) -> &'static str {
    match page {
        Page::Splash => "",
        Page::Main => "SCUTOONS DECK",
        Page::Live => "LIVE",
        Page::Scenes => SCENES,
        Page::Settings => "SETTINGS",
        Page::Pair => "PAIR THIS DECK",
        Page::Avatar => "AVATAR",
        Page::Equip => "EQUIP",
        Page::Mappings => "MAPPINGS",
        Page::Start | Page::Waiting => "SCUTOONS DECK",
        Page::Play => "PLAY",
        Page::Show => "SHOW",
        Page::Join => "JOIN A SHOW",
        Page::Shutdown => "SHUTDOWN",
    }
}

impl Menu {
    /// What to show now on a `w × h` screen (device pixels): None = as before; Some(None) = nothing (closed, or the
    /// deck's own page shows); Some(layer) = this page over the whole screen, from its top left (the screen's menu
    /// layer, painted through egui: deck-space `Stage::set_menu`).
    pub fn paint(&mut self, f: &Facts, (w, h): (f32, f32)) -> Option<Option<Layer>> {
        let show = self.top().filter(|_| self.own(f).is_none() || f.sheet.is_some());
        let now: Option<Drawn> = show.map(|_| (self.stack.clone(), self.cursor, f.clone(), (w, h), self.waiting.clone(), self.join.clone(), self.changing()));
        let was: Option<Drawn> = self.drawn.take();
        if now == was {
            self.drawn = was;
            return None;
        }
        self.drawn = now;
        self.places.clear();
        let Some(top) = show else { return Some(None) };
        let s = scale(w, h, f.density);
        let (lw, lh) = ((w / s).ceil(), (h / s).ceil());
        let finger = finger(lw, lh);
        let mut d = look::list();
        self.places = self.draw(&mut d, top, f, (lw, lh, s), finger);
        self.scale = s;
        // a wide screen's page lies on a grid of one pixel a cell, each grown to `s` (the HUD's look); a finger's small
        // values need the screen's own pixels
        let k = if finger { s } else { 1.0 };
        Some(Some(look::layer(d, s, k)))
    }

    /// The page `top` into `d` over a screen of `lw × lh` logical pixels; what can be pressed.
    fn draw(&self, d: &mut DrawList, top: Page, f: &Facts, (lw, lh, s): (f32, f32, f32), finger: bool) -> Vec<(Rect, Hit)> {
        let t = deck_kit::live::tokens::hud();
        if top == Page::Splash {
            splash(d, &t, f, (lw, lh));
            return vec![];
        }
        // the deck's own pages: their 38 columns after the row's `>`
        let wide = if top.own() && f.sheet.is_some() { COLUMNS + 2.0 * GLYPH } else { COLUMNS };
        let w = if finger { lw - 2.0 * PAD - 8.0 } else { wide.min(lw - 2.0 * PAD - 8.0) };
        // the body once to measure it, then on its ground in the middle of what the host leaves free
        let mut scratch = look::list();
        let height = self.body(&mut Body { d: &mut scratch, t: &t, x: 0.0, w, finger, places: vec![] }, top, f, 0.0) + 2.0 * PAD;
        let free = (f.top / s).min(lh);
        let at = Rect { x: ((lw - w) / 2.0 - PAD).max(0.0), y: (free + (lh - free - height) / 2.0).max(free.min(lh)), w: w + 2.0 * PAD, h: height };
        look::ground(d, &t, Some((lw, lh)), at);
        let mut b = Body { d, t: &t, x: at.x + PAD, w, finger, places: vec![] };
        self.body(&mut b, top, f, at.y + PAD);
        b.places
    }

    /// A page's body from `y`; its height.
    fn body(&self, b: &mut Body, top: Page, f: &Facts, y: f32) -> f32 {
        let y0 = y;
        let mut y = y;
        match top {
            Page::Main | Page::Start => {
                y += b.head(y, title(top), if f.native { "ESC CLOSE" } else { "M CLOSE" });
                let under = if top == Page::Start { format!("{} · {}", f.pair.word(), f.session.as_deref().unwrap_or("NO SESSION")) } else { f.session.clone().unwrap_or("NO SESSION".into()) };
                y += b.line(y, &under, "", true);
                y += b.rule(y);
                y += self.items(b, top, f, y);
                y += b.rule(y);
                if b.finger {
                    y += b.foot_back(y, f);
                } else {
                    y += b.line(y, "UP DOWN CHOOSE  ENTER OPEN", "", true);
                    if f.native {
                        y += b.line(y, "F4 SETTINGS F5 ON THIS DECK F6 LIVE", "", true);
                    }
                }
            }
            Page::Play => {
                y += b.head(y, title(top), "ESC BACK");
                y += b.line(y, "DEMOS FIRST, THEN THE PUBLISHED ONES", "", true);
                y += b.rule(y);
                if f.play.is_empty() {
                    y += b.line(y, "NOTHING TO PLAY: OFFLINE AND NO SCENE KEPT", "", true);
                } else {
                    y += self.items(b, top, f, y);
                }
                y += b.rule(y);
                if b.finger {
                    y += b.foot_back(y, f);
                } else {
                    y += b.line(y, "UP DOWN CHOOSE  ENTER PLAY", "", true);
                }
            }
            Page::Join => {
                y += b.head(y, title(top), "ESC BACK");
                y += b.line(y, "CODE FROM THE CONDUCTOR'S SESSION SHEET", "", true);
                y += b.rule(y);
                y += self.join.draw(b.d, b.t, (b.x, y, b.w), b.finger, &mut b.places);
                if b.finger {
                    y += b.foot_back(y, f);
                }
            }
            Page::Waiting => {
                y += b.head(y, title(top), "");
                y += b.rule(y);
                y += b.line(y, &crate::start::waiting_line(&self.waiting).to_uppercase(), "", false);
                y += b.line(y, "THE CONDUCTOR IS NOT IN THE SESSION", "", true);
                y += b.rule(y);
                y += self.items(b, top, f, y);
                y += b.rule(y);
                if !b.finger {
                    y += b.line(y, "ENTER LEAVE  ESC CLOSE", "", true);
                }
            }
            Page::Show => {
                y += b.head(y, title(top), "ESC CLOSE");
                let showing = f.showing.clone().unwrap_or_default();
                y += b.line(y, &showing.name.to_uppercase(), &showing.line(), true);
                y += b.line(y, "THIS DECK IS ITS CONDUCTOR", "", true);
                y += b.rule(y);
                y += self.items(b, top, f, y);
                y += b.rule(y);
                if b.finger {
                    y += b.foot_back(y, f);
                } else {
                    y += b.line(y, "ENTER GO  ESC CLOSE", "", true);
                }
            }
            Page::Shutdown => {
                y += b.head(y, title(top), "ESC CANCEL");
                y += b.rule(y);
                y += b.line(y, "SHUT THE DECK DOWN?", "", false);
                y += b.line(y, "IT STAYS OFF UNTIL ITS POWER BUTTON", "", true);
                y += b.rule(y);
                y += self.items(b, top, f, y);
                y += b.rule(y);
                if b.finger {
                    y += b.foot_back(y, f);
                } else {
                    y += b.line(y, "ENTER (A) SHUT DOWN  ESC (B) CANCEL", "", true);
                }
            }
            Page::Pair => {
                y += b.head(y, title(top), "ESC BACK");
                y += b.line(y, f.pair.word(), "", true);
                y += b.rule(y);
                y += pair(b, f, y);
                y += b.rule(y);
                if let Pair::Waiting { .. } = f.pair {
                    y += b.line(y, "WAITING FOR A MEMBER ...", "", true);
                }
                if b.finger {
                    y += b.foot_back(y, f);
                }
            }
            p if p.own() && f.sheet.is_some() => {
                let sheet = f.sheet.clone().unwrap_or_default();
                y += b.head(y, title(p), "ESC BACK");
                y += b.rule(y);
                y += sheet_lines(b, &sheet, y);
                y += b.rule(y);
                if b.finger {
                    y += b.foot_back(y, f);
                } else {
                    for keys in keys_of(p, self.changing()) {
                        y += b.line(y, keys, "", true);
                    }
                }
            }
            // a later page, or one of the deck's own pages this deck has not: empty, the boilerplate's slot
            _ => {
                y += b.head(y, title(top), "ESC BACK");
                y += b.rule(y);
                y += b.line(y, "NOTHING HERE YET", "", true);
                if b.finger {
                    y += b.rule(y);
                    y += b.foot_back(y, f);
                }
            }
        }
        y - y0
    }
}

impl Menu {
    /// A list page's rows from `y` (Play's by the scene's name), each a place to press; their height.
    fn items(&self, b: &mut Body, top: Page, f: &Facts, y: f32) -> f32 {
        let mut h = 0.0;
        for (i, row) in rows_of(top, f).into_iter().enumerate() {
            let (label, value, dim) = words(row, f);
            let name = match row {
                super::Row::Pick(n) => f.play.get(n).map(|p| p.label.to_uppercase()).unwrap_or_default(),
                _ => label.to_string(),
            };
            let at = look::item(b.d, b.t, (b.x, y + h, b.w), Item { label: &name, value: &value, chosen: i == self.cursor, dim }, b.finger);
            b.places.push((at, Hit::Row(i)));
            h += at.h;
        }
        h
    }
}

/// The lines of the deck's own page from `y`, the cursor's chosen, each a place to point at; their height.
fn sheet_lines(b: &mut Body, sheet: &Sheet, y: f32) -> f32 {
    let mut h = 0.0;
    for (i, l) in sheet.lines.iter().enumerate() {
        let at = look::item(b.d, b.t, (b.x, y + h, b.w), Item { label: l, value: "", chosen: i == sheet.cursor, dim: false }, b.finger);
        b.places.push((at, Hit::Line(i)));
        h += at.h;
    }
    h
}

/// the keys of the deck's own page on a wide screen: the keyboard's and the pad's
fn keys_of(p: Page, changing: bool) -> &'static [&'static str] {
    match p {
        Page::Settings if changing => &["CHANGING: < > VALUE  ENTER (A) DONE"],
        Page::Scenes => &["ENTER (A) PLAY  S START  ESC (B) BACK"],
        Page::Settings => &["ENTER (A) CHANGE  + - P=PX R=RESET", "ESC (B) BACK"],
        Page::Live => &["ENTER (A) TAKE  TYPE A NAME  (B) BACK"],
        _ => &[],
    }
}

/// The pairing's body: the QR and the code (beside it on a wide screen, under it on a finger's), or the state's lines.
fn pair(b: &mut Body, f: &Facts, y: f32) -> f32 {
    let lines = |b: &mut Body, said: &[&str]| said.iter().enumerate().map(|(i, l)| b.line(y + i as f32 * ROW, l, "", false)).sum::<f32>();
    match &f.pair {
        Pair::Waiting { code, hint, qr } => {
            let room = if b.finger { b.w } else { b.w * 0.45 };
            let module = (room.min(160.0) / (qr.n + 8) as f32).floor().max(1.0);
            let side = look::qr(b.d, (b.x, y), module, qr.n, &|x, y| qr.dark.get(y * qr.n + x).copied().unwrap_or(false));
            let (tx, mut ty) = if b.finger { (b.x, y + side + 4.0) } else { (b.x + side + 12.0, y + 4.0) };
            let tw = b.x + b.w - tx;
            ty += look::line(b.d, b.t, (tx, ty, tw), "CODE", "", true);
            ty += look::code(b.d, b.t, (tx, ty), code);
            ty += look::line(b.d, b.t, (tx, ty, tw), "ON YOUR PHONE:", "", true);
            // the link broken after its slashes, where it does not fit
            let mut part = String::new();
            for piece in hint.split_inclusive('/') {
                if !part.is_empty() && look::cols(b.d, tw) < part.chars().count() + piece.chars().count() {
                    ty += look::line(b.d, b.t, (tx, ty, tw), &part, "", false);
                    part.clear();
                }
                part.push_str(piece);
            }
            ty += look::line(b.d, b.t, (tx, ty, tw), &part, "", false);
            (ty - y).max(side) + 2.0
        }
        Pair::Paired(to) if to.is_empty() => lines(b, &["THIS DECK IS PAIRED"]),
        Pair::Paired(to) => lines(b, &[&format!("PAIRED TO {to}")]),
        Pair::SignedIn => {
            look::line(b.d, b.t, (b.x, y, b.w), "THIS DECK IS YOUR ACCOUNT", "", false);
            look::line(b.d, b.t, (b.x, y + ROW, b.w), "NO PAIRING NEEDED", "", true) + ROW
        }
        Pair::Offline(why) => {
            let said = crate::wrap(why, look::cols(b.d, b.w));
            for (i, l) in said.iter().enumerate() {
                look::line(b.d, b.t, (b.x, y + i as f32 * ROW, b.w), l, "", false);
            }
            said.len() as f32 * ROW
        }
        Pair::Off => lines(b, &["NO PAIRING ON THIS DECK"]),
    }
}

/// The splash's words over the logo (the stage draws the logo): the loading line, PRESS START, its keys, and at the
/// bottom whether the deck is paired – the corner card's place is gone (owner 2026-11-11: "Die pairing karte muss ins
/// haupt menu").
fn splash(d: &mut DrawList, t: &Tokens, f: &Facts, (lw, lh): (f32, f32)) {
    // under the logo: the stage draws it half the screen wide in the middle (deck-space splash.wgsl)
    let below = (lh * 0.5 + lw * 0.25 / LOGO_ASPECT).min(lh - 6.0 * ROW);
    if f.loading {
        look::centred(d, t, (0.0, below + ROW, lw), GLYPH, "LOADING ...", true);
    }
    look::centred(d, t, (0.0, below + 3.0 * ROW, lw), 2.0 * GLYPH, "PRESS START", false);
    look::centred(d, t, (0.0, below + 5.0 * ROW + 2.0, lw), GLYPH, "ENTER · (A)", false);
    look::line(d, t, (PAD, lh - ROW - PAD, lw - 2.0 * PAD), "", &format!("{} · NO SESSION", f.pair.word()), true);
}

#[cfg(test)]
mod tests {
    use super::super::{Answer, Input, Qr};
    use super::*;
    use deck_kit::base::draw::Prim;

    fn native() -> Facts {
        Facts { native: true, live_page: true, scenes_page: true, settings_page: true, density: 1.0, scene: "fireflies".into(), ..Facts::default() }
    }

    #[test]
    fn the_scale_is_the_huds_at_1080p_and_a_phones_sixteen_css_pixels() {
        assert_eq!((scale(1920.0, 1080.0, 1.0), scale(1280.0, 720.0, 1.0), scale(750.0, 1624.0, 2.0), scale(1024.0, 768.0, 1.0)), (2.0, 2.0, 4.0, 2.0));
        assert!(!finger(960.0, 540.0) && finger(188.0, 406.0));
    }

    #[test]
    fn the_menu_is_painted_once_and_a_press_on_a_row_takes_it() {
        let f = native();
        let mut m = Menu::new(true);
        assert_eq!(m.paint(&f, (1920.0, 1080.0)), None, "closed and nothing shown");
        m.input(Input::Open, &f);
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("the main menu");
        assert_eq!((layer.scale, layer.grid), (2.0, 1.0), "a pixel a cell, grown to twice its size");
        assert_eq!(m.paint(&f, (1920.0, 1080.0)), None, "nothing changed");
        // the rows' places: 10 high, the box 336 wide in the middle
        let rows: Vec<Rect> = m.places.iter().filter(|(_, h)| matches!(h, Hit::Row(_))).map(|(r, _)| *r).collect();
        assert_eq!(rows.len(), 9);
        assert!(rows.iter().all(|r| r.h == ROW && r.w == COLUMNS) && rows[1].y == rows[0].y + ROW);
        assert_eq!(rows[0].x, (960.0 - COLUMNS) / 2.0);
        // the chosen row wears the red bar
        let red = Prim::Rect { x: rows[0].x, y: rows[0].y, w: rows[0].w, h: ROW, r: 0.0, color: [232.0 / 255.0, 48.0 / 255.0, 42.0 / 255.0, 1.0] };
        assert!(layer.list.prims().contains(&red));
        // hovering the fourth row chooses it, a press on the second opens LIVE (the deck's own page: nothing painted)
        m.input(Input::Point((rows[3].x + 4.0) * 2.0, (rows[3].y + 4.0) * 2.0), &f);
        assert_eq!(m.cursor(), 3);
        m.input(Input::Press((rows[1].x + 4.0) * 2.0, (rows[1].y + 4.0) * 2.0), &f);
        assert_eq!(m.own(&f), Some(Page::Live));
        assert_eq!(m.paint(&f, (1920.0, 1080.0)), Some(None));
    }

    /// the start menu and the wait are painted with their rows; a press on PLAY opens Play, on LEAVE leaves
    #[test]
    fn the_start_menu_and_the_wait_are_painted_and_pressed() {
        use crate::start::{Playable, Stand};
        let text = |layer: &Layer, want: &str| layer.list.prims().iter().any(|p| matches!(*p, Prim::Text { start, end, .. } if layer.list.text_of(start, end) == want));
        let f = Facts { play: vec![Playable { value: "scene:sandy-dunes".into(), label: "Dunes".into(), demo: true, show: false }], ..native() };
        let mut m = Menu::default();
        m.follow(&Stand::Menu);
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("the start menu");
        assert!(text(&layer, "PLAY") && text(&layer, "JOIN A SHOW") && text(&layer, "1 SCENES"));
        let rows: Vec<Rect> = m.places.iter().filter(|(_, h)| matches!(h, Hit::Row(_))).map(|(r, _)| *r).collect();
        assert_eq!(rows.len(), 4, "play, join, settings, quit");
        m.input(Input::Press((rows[0].x + 4.0) * 2.0, (rows[0].y + 4.0) * 2.0), &f);
        assert_eq!(m.top(), Some(Page::Play));
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("play");
        assert!(text(&layer, "DUNES") && text(&layer, "DEMO"));
        m.follow(&Stand::Waiting("CHOIR".into()));
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("the wait");
        assert!(text(&layer, "WAITING FOR CHOIR") && text(&layer, "LEAVE"));
        let (leave, _) = *m.places.iter().find(|(_, h)| *h == Hit::Row(0)).expect("leave");
        assert_eq!(m.input(Input::Press((leave.x + 4.0) * 2.0, (leave.y + 4.0) * 2.0), &f), Answer::Leave);
    }

    /// headless, each of the deck's own pages: painted in the menu's middle under its name, its cursor's line chosen; the
    /// pointer on a line hands it to the page; the main menu's PLAY and the shutdown's question painted too
    #[test]
    fn the_decks_own_pages_are_painted_in_the_middle_and_pointed_at() {
        use super::super::Sheet;
        let text = |layer: &Layer, want: &str| layer.list.prims().iter().any(|p| matches!(*p, Prim::Text { start, end, .. } if layer.list.text_of(start, end) == want));
        let sheet = Sheet { lines: vec!["  IDLE".into(), ">*CATHEDRAL".into(), " -- PUBLISHED --".into(), "  NAVE".into()], cursor: 1 };
        let f = Facts { play_page: true, power: true, sheet: Some(sheet), ..native() };
        for (page, name) in [(Page::Scenes, "ON THIS DECK"), (Page::Settings, "SETTINGS"), (Page::Live, "LIVE")] {
            let mut m = Menu::new(true);
            m.direct(page);
            let layer = m.paint(&f, (1920.0, 1080.0)).flatten().unwrap_or_else(|| panic!("{page:?} painted by the menu"));
            assert!(text(&layer, name) && text(&layer, ">*CATHEDRAL") && text(&layer, "ESC BACK"), "{page:?}");
            let lines: Vec<Rect> = m.places.iter().filter(|(_, h)| matches!(h, Hit::Line(_))).map(|(r, _)| *r).collect();
            assert_eq!(lines.len(), 4);
            let w = COLUMNS + 2.0 * GLYPH;
            assert!(lines.iter().all(|r| r.x == (960.0 - w) / 2.0 && r.w == w), "in the middle, two columns wider than the menu");
            let red = Prim::Rect { x: lines[1].x, y: lines[1].y, w: lines[1].w, h: ROW, r: 0.0, color: [232.0 / 255.0, 48.0 / 255.0, 42.0 / 255.0, 1.0] };
            assert!(layer.list.prims().contains(&red), "the cursor's line wears the red bar");
            let (x, y) = ((lines[3].x + 4.0) * 2.0, (lines[3].y + 4.0) * 2.0);
            assert_eq!(m.input(Input::Point(x, y), &f), Answer::Line(page, 3, false));
            assert_eq!(m.input(Input::Press(x, y), &f), Answer::Line(page, 3, true));
        }
        let mut m = Menu::new(true);
        m.input(Input::Open, &f);
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("the main menu");
        assert!(text(&layer, "PLAY") && text(&layer, "ON THIS DECK") && text(&layer, "RESTART") && text(&layer, "SHUTDOWN") && !text(&layer, "QUIT"));
        m.direct(Page::Shutdown);
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("the question");
        assert!(text(&layer, "SHUT THE DECK DOWN?") && text(&layer, "SHUT DOWN"));
    }

    #[test]
    fn a_phone_gets_a_fingers_rows_and_the_back_line_goes_back() {
        let f = Facts { live_page: true, density: 2.0, ..Facts::default() };
        let mut m = Menu::new(true);
        m.input(Input::Open, &f);
        let layer = m.paint(&f, (750.0, 1624.0)).flatten().expect("the menu");
        assert_eq!((layer.scale, layer.grid), (4.0, 4.0), "the screen's own pixels");
        let rows: Vec<Rect> = m.places.iter().filter(|(_, h)| matches!(h, Hit::Row(_))).map(|(r, _)| *r).collect();
        assert!(rows.len() == 8 && rows.iter().all(|r| r.h == 20.0), "40 px at 16 px glyphs, no QUIT: {rows:?}");
        let (back, _) = *m.places.iter().find(|(_, h)| *h == Hit::Back).expect("the back line");
        m.input(Input::Press((back.x + 2.0) * 4.0, (back.y + 2.0) * 4.0), &f);
        assert!(!m.open());
    }

    #[test]
    fn the_pair_page_shows_the_code_and_its_qr() {
        let mut f = native();
        f.pair = Pair::Waiting { code: "Q5H-RT2".into(), hint: "scutoons.com/account/decks".into(), qr: Qr { n: 21, dark: (0..441).map(|i| i % 3 == 0).collect() } };
        let mut m = Menu::new(true);
        m.input(Input::Open, &f);
        m.direct(Page::Pair);
        let layer = m.paint(&f, (1920.0, 1080.0)).flatten().expect("the pair page");
        let rects = |c: [f32; 4]| layer.list.prims().iter().filter(|p| matches!(p, Prim::Rect { color, .. } if *color == c)).count();
        let (white, black) = (rects([1.0; 4]), rects([0.0, 0.0, 0.0, 1.0]));
        assert!(white == 1 && black >= 147, "the QR's square and its 147 modules: {white} white, {black} black");
        let mint = deck_kit::live::tokens::HUD_ACCENT;
        let code = layer.list.prims().iter().any(|p| matches!(*p, Prim::Text { start, end, color, .. } if color == mint && layer.list.text_of(start, end) == "Q5H-RT2"));
        assert!(code, "the code in the accent");
    }
}
