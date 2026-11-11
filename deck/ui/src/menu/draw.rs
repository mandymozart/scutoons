//! The menu drawn (#386, draft 1): every page laid out top down with the kit's live components (deck-kit `live/hud.rs`,
//! the HUD's arcade look) and painted by the kit through egui – one layer for the native deck and the browser deck, a
//! full screen the stage lays over the scene (deck-space `Stage::set_menu`, #485). Sizes are logical pixels (a cell of the pixel face): the
//! HUD's scale makes 16 px glyphs on a 1080p screen, 40 columns; on a phone (narrow, upright) the rows are a finger's –
//! 20 high, 40 px at 16 px glyphs, a value under its label. What can be pressed is kept for the pointer (`places`).
use super::{rows, words, Facts, Hit, Menu, Page, Pair};
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

/// what was drawn: the pages, the cursor, the facts, the screen (`Menu::drawn`)
pub(crate) type Drawn = (Vec<Page>, usize, Facts, (f32, f32));

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
        Page::Scenes => "SCENES",
        Page::Settings => "SETTINGS",
        Page::Pair => "PAIR THIS DECK",
        Page::Avatar => "AVATAR",
        Page::Equip => "EQUIP",
        Page::Mappings => "MAPPINGS",
    }
}

impl Menu {
    /// What to show now on a `w × h` screen (device pixels): None = as before; Some(None) = nothing (closed, or the
    /// deck's own page shows); Some(layer) = this page over the whole screen, from its top left (the screen's menu
    /// layer, painted through egui: deck-space `Stage::set_menu`).
    pub fn paint(&mut self, f: &Facts, (w, h): (f32, f32)) -> Option<Option<Layer>> {
        let show = self.top().filter(|_| self.own(f).is_none());
        let now: Option<Drawn> = show.map(|_| (self.stack.clone(), self.cursor, f.clone(), (w, h)));
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
        let w = if finger { lw - 2.0 * PAD - 8.0 } else { COLUMNS.min(lw - 2.0 * PAD - 8.0) };
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
            Page::Main => {
                y += b.head(y, title(top), if f.native { "ESC CLOSE" } else { "M CLOSE" });
                y += b.line(y, f.session.as_deref().unwrap_or("NO SESSION"), "", true);
                y += b.rule(y);
                for (i, row) in rows(f).into_iter().enumerate() {
                    let (label, value, dim) = words(row, f);
                    let at = look::item(b.d, b.t, (b.x, y, b.w), Item { label, value: &value, chosen: i == self.cursor, dim }, b.finger);
                    b.places.push((at, Hit::Row(i)));
                    y += at.h;
                }
                y += b.rule(y);
                if b.finger {
                    y += b.foot_back(y, f);
                } else {
                    y += b.line(y, "UP DOWN CHOOSE  ENTER OPEN", "", true);
                    if f.native {
                        y += b.line(y, "F4 SETTINGS F5 SCENES F6 LIVE", "", true);
                    }
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
    use super::super::{Input, Qr};
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
        let mint = deck_kit::live::tokens::hud().color.accent;
        let code = layer.list.prims().iter().any(|p| matches!(*p, Prim::Text { start, end, color, .. } if color == mint && layer.list.text_of(start, end) == "Q5H-RT2"));
        assert!(code, "the code in the accent");
    }
}
