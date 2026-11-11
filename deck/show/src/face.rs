//! The deck's dialogs as a picture, and the presses back (#481; owner 2026-11-11: "Ja use the crate in core"): what
//! `flashed.rs` says to the person at the deck (`dialog.rs`), laid out with the component library's panel (deck-kit
//! `panel.rs`, the reader's style) into a layer the kit paints through egui (#485) – one drawing for the native deck and
//! the browser deck. A platform hands in the screen's size and the deck's mono face's advance, shows the layer over its
//! stage (deck-space `Stage::set_panel`: along the bottom), and hands taps and keys back. No window and no GPU here.
use crate::dialog::{Key, Row, Shown};
use crate::flashed::Flashed;
#[cfg(test)]
use crate::ready;
use deck_kit::base::draw::{DrawList, Rect};
pub use deck_kit::base::screen::Layer;
use deck_kit::live::panel::{self, Part};
use deck_space::fetched::Fetched;

#[derive(Debug, Clone, Default)]
pub struct Face {
    /// what is painted now: its words and the screen (none: nothing, or to paint again)
    drawn: Option<String>,
    /// where what can be pressed is, in logical pixels: a dialog's rows, then its buttons; a toast
    places: Vec<Rect>,
    /// device pixels per logical one, as painted
    scale: f32,
    /// where what shows begins, in device pixels from the screen's top
    top: f32,
    /// the person uses keys: the dialog shows what they are on (no line says what the keys do: owner 2026-11-11)
    keys: bool,
}

impl Face {
    /// What to show now on a `w × h` screen (device pixels), in the deck's mono face (`per_px`: its advance per pixel of
    /// font size): None = as before; Some(None) = nothing any more; Some(layer) = this, along the screen's bottom.
    pub fn paint(&mut self, shown: Option<&Shown>, (w, h): (f32, f32), per_px: f32) -> Option<Option<Layer>> {
        let drawn = shown.map(|s| format!("{s:?} {w}x{h} {}", self.keys));
        if drawn == self.drawn {
            return None;
        }
        self.drawn = drawn;
        self.places.clear();
        let Some(shown) = shown else { return Some(None) };
        let scale = panel::scale(w, h);
        let screen = (w / scale, h / scale);
        let tokens = deck_kit::live::tokens::panel();
        let mut d = DrawList::new(per_px);
        let top = match shown {
            Shown::Dialog(dialog) => {
                let labels: Vec<&str> = dialog.buttons.iter().map(|(label, _)| label.as_str()).collect();
                let rows = dialog.rows.len();
                let mut parts = vec![Part::Caption(&dialog.caption.0, &dialog.caption.1)];
                if !dialog.title.is_empty() {
                    parts.push(Part::Title(&dialog.title));
                }
                parts.extend(dialog.rows.iter().enumerate().map(|(i, row)| match row {
                    Row::Switch(label, on) => Part::Switch(label, *on, self.keys && dialog.focus == i),
                }));
                parts.extend(dialog.lines.iter().map(|line| Part::Line(line, true)));
                parts.push(Part::Buttons(&labels, dialog.focus.checked_sub(rows).filter(|_| self.keys), dialog.gone));
                self.places = panel::draw(&mut d, &tokens, screen, &parts, !self.keys);
                // the glow above the edge is part of what shows
                screen.1 - panel::height(&d, &tokens, screen.0, &parts) - 16.0
            }
            Shown::Toast(text, tone) => {
                let at = panel::toast(&mut d, &tokens, screen, text, *tone);
                self.places.push(at);
                at.y - 2.0
            }
        }
        .max(0.0);
        (self.scale, self.top) = (scale, top * scale);
        Some(Some(panel::layer(d, scale)))
    }

    /// Where what shows begins, in device pixels from the screen's top (a platform that cannot ask `tap` at once
    /// knows by it whether a tap may be the deck's UI's); none = nothing shows.
    pub fn top(&self) -> Option<f32> {
        self.drawn.as_ref().map(|_| self.top)
    }

    /// The stage was made anew: what is shown is painted again with the next `paint`.
    pub fn again(&mut self) {
        self.drawn = None;
    }

    /// A tap or a click at (x, y) of the screen, in device pixels: what is there is pressed. true = it was on something.
    pub fn tap(&mut self, x: f32, y: f32, flashed: &mut Flashed, fetched: &mut Fetched) -> bool {
        let (lx, ly) = (x / self.scale.max(1.0), y / self.scale.max(1.0));
        // what was painted last may be gone already (a toast's time ran out before the next picture)
        let Some(index) = self.places.iter().position(|at| at.has(lx, ly)).filter(|_| flashed.shown().is_some()) else { return false };
        self.keys = false;
        flashed.press(index, fetched);
        true
    }

    /// A key, by the name the web gives it (`KeyboardEvent.key`: `ArrowLeft`, `Enter`, `Tab`, a character …; the native
    /// deck names its keys the same). While a dialog shows, the arrows, Tab, Enter and Escape are its own; with no
    /// dialog, `f` opens "please don't flash" for who may set it. true = the key was the deck's UI's.
    pub fn key(&mut self, name: &str, flashed: &mut Flashed, fetched: &mut Fetched) -> bool {
        let dialog = matches!(flashed.shown(), Some(Shown::Dialog(_)));
        let key = match name {
            "ArrowLeft" => Key::Left,
            "ArrowRight" => Key::Right,
            "ArrowUp" => Key::Up,
            "ArrowDown" | "Tab" => Key::Down,
            "Enter" => Key::Enter,
            "Escape" => Key::Escape,
            "f" | "F" if !dialog && flashed.open_keep() => {
                self.keys = true;
                return true;
            }
            _ => return false,
        };
        if !dialog {
            return false;
        }
        // the first key only shows where the keys are
        if self.keys {
            flashed.key(key, fetched);
        }
        self.keys = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_kit::base::draw::Prim;
    use protocol::Flash;

    /// the deck's mono face's advance (IBM Plex Mono: 0.6 em)
    fn font() -> f32 {
        0.6
    }

    /// what a layer covers, in device pixels: its leftmost, topmost, rightmost and lowest edge
    fn extent(l: &Layer) -> [f32; 4] {
        l.list.prims().iter().fold([f32::MAX, f32::MAX, 0.0, 0.0], |[x0, y0, x1, y1], p| match *p {
            Prim::Rect { x, y, w, h, .. } | Prim::Outline { x, y, w, h, .. } => [x0.min(x * l.scale), y0.min(y * l.scale), x1.max((x + w) * l.scale), y1.max((y + h) * l.scale)],
            _ => [x0, y0, x1, y1],
        })
    }

    fn flash(id: &str, version: u64) -> Flash {
        let top = protocol::Pinned { id: "top".into(), version };
        Flash { root: top.clone(), shows: vec![top], lock: None, hops: 0, here: None, id: Some(id.into()), by: Some("ana".into()), skip: None, force: None, needs: Some(Default::default()) }
    }

    const SCREEN: (f32, f32) = (800.0, 600.0);

    #[test]
    fn a_suggestion_is_painted_once_and_a_tap_on_its_second_button_rejects() {
        let (mut face, mut f, mut fetched, font) = (Face::default(), Flashed::new("left", "browser"), Fetched::default(), font());
        f.joined("c1");
        f.set_may_reject(true);
        assert_eq!(face.paint(f.shown().as_ref(), SCREEN, font), None, "nothing shows and nothing showed");
        f.flash(&flash("f1", 4), "", "no token", &mut fetched);
        // 800 × 600 is two device pixels per logical one: the panel is as wide as the screen, along its bottom
        let strip = face.paint(f.shown().as_ref(), SCREEN, font).flatten().expect("the suggestion");
        let [x0, y0, x1, y1] = extent(&strip);
        assert!(strip.scale == 2.0 && x0 == 0.0 && x1 == 800.0 && y1 == 600.0 && y0 > 0.0 && y0 < 400.0, "{:?}", extent(&strip));
        assert_eq!(face.paint(f.shown().as_ref(), SCREEN, font), None, "nothing changed");
        // outside the buttons: not the dialog's; on the right half of the buttons' row: reject
        assert!(!face.tap(10.0, 10.0, &mut f, &mut fetched));
        let reject = face.places[1];
        assert!(face.tap((reject.x + 5.0) * 2.0, (reject.y + 5.0) * 2.0, &mut f, &mut fetched));
        assert!(f.reports().iter().any(|r| !r.accepted && r.why.as_deref() == Some("rejected at the deck")));
        // the toast: a small box at the bottom left, gone on a tap
        assert!(matches!(f.shown(), Some(Shown::Toast(..))));
        let toast = face.paint(f.shown().as_ref(), SCREEN, font).flatten().expect("the toast");
        let [x0, y0, x1, y1] = extent(&toast);
        assert!(y1 - y0 < 140.0 && x0 < 40.0 && x1 < 799.0, "a small box, nothing beside it: {:?}", extent(&toast));
        let at = face.places[0];
        assert!(face.tap((at.x + 5.0) * 2.0, (at.y + 5.0) * 2.0, &mut f, &mut fetched));
        // nothing shows any more, though the picture was not painted again yet: a tap there is not the deck's UI's
        assert!(f.shown().is_none() && !face.tap((at.x + 5.0) * 2.0, (at.y + 5.0) * 2.0, &mut f, &mut fetched));
        assert_eq!(face.paint(f.shown().as_ref(), SCREEN, font), Some(None));
    }

    #[test]
    fn please_dont_flash_is_opened_switched_and_saved_with_keys() {
        let (mut face, mut f, mut fetched, font) = (Face::default(), Flashed::new("left", "browser"), Fetched::default(), font());
        f.joined("c1");
        f.reports();
        // not for someone who may not reject
        assert!(!face.key("f", &mut f, &mut fetched));
        f.set_may_reject(true);
        f.set_locale("de");
        assert!(!face.key("x", &mut f, &mut fetched), "any other key is the page's");
        assert!(face.key("f", &mut f, &mut fetched));
        let Some(Shown::Dialog(d)) = f.shown() else { panic!("the dialog") };
        assert_eq!((d.rows.clone(), d.buttons.len(), d.focus), (vec![Row::Switch("Bitte nicht flashen".into(), false)], 2, 0));
        assert!(!face.key("x", &mut f, &mut fetched), "a character is not the dialog's: nothing is typed");
        assert!(face.paint(f.shown().as_ref(), SCREEN, font).flatten().is_some());
        // Enter flips the switch, Tab goes to Save, Enter saves: the conductor hears it at once, the person reads a toast
        for key in ["Enter", "Tab", "Enter"] {
            assert!(face.key(key, &mut f, &mut fetched), "{key}");
        }
        assert_eq!(f.deck.keep(), Some(ready::NO_REASON));
        assert_eq!(f.reports().last().and_then(|r| r.keep.clone()).as_deref(), Some(ready::NO_REASON));
        assert!(matches!(f.shown(), Some(Shown::Toast(text, _)) if text == "Gespeichert: bitte nicht flashen"));
        // a deck that asks not to be flashed is not asked about a suggestion: it refuses by itself
        f.flash(&flash("f2", 4), "", "no token", &mut fetched);
        assert!(f.reports().iter().any(|r| !r.accepted) && !matches!(f.shown(), Some(Shown::Dialog(_))));
        // opened again it shows what stands; Escape leaves it
        assert!(face.key("f", &mut f, &mut fetched));
        assert!(matches!(f.shown(), Some(Shown::Dialog(d)) if d.rows[0] == Row::Switch("Bitte nicht flashen".into(), true)));
        assert!(face.key("Escape", &mut f, &mut fetched));
        assert_eq!(f.deck.keep(), Some(ready::NO_REASON));
        // its refusal and its line name no reason
        assert_eq!(f.deck.last().and_then(|r| r.why.clone()).as_deref(), Some("asks not to be flashed"));
        // a deck that came with a reason (a native deck's deck.toml) keeps it through the switch
        f.deck.set_keep(Some("Probe läuft".into()));
        for key in ["f", "Tab", "Enter"] {
            assert!(face.key(key, &mut f, &mut fetched), "{key}");
        }
        assert_eq!(f.deck.keep(), Some("Probe läuft"));
    }
}
