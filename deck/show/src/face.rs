//! The deck's dialogs as a picture, and the presses back (#481; owner 2026-11-11: "Ja use the crate in core"): what
//! `flashed.rs` says to the person at the deck (`dialog.rs`), laid out with the component library's panel (deck-kit
//! `panel.rs`, the reader's style) and painted on the CPU by its `raster` – one drawing for the native deck and the
//! browser deck. A platform hands in the screen's size and the deck's mono face, shows the pixels as one more overlay
//! of its stage (deck-space `Stage::set_panel`: a strip along the bottom), and hands taps, keys and typed text back.
//! No window and no GPU here.
use crate::dialog::{Key, Row, Shown};
use crate::flashed::Flashed;
use deck_kit::controls::Rect;
use deck_kit::draw::DrawList;
use deck_kit::panel::{self, Part};
use deck_kit::raster::{self, Font};
pub use deck_kit::raster::Pixels;
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
    /// the person uses keys: the dialog shows what they are on and what they do
    keys: bool,
}

impl Face {
    /// What to show now on a `w × h` screen (device pixels), in the deck's mono `font`: None = as before; Some(None) =
    /// nothing any more; Some(pixels) = this strip, as wide as the screen, along its bottom.
    pub fn paint(&mut self, shown: Option<&Shown>, (w, h): (f32, f32), font: &impl Font) -> Option<Option<Pixels>> {
        let drawn = shown.map(|s| format!("{s:?} {w}x{h} {}", self.keys));
        if drawn == self.drawn {
            return None;
        }
        self.drawn = drawn;
        self.places.clear();
        let Some(shown) = shown else { return Some(None) };
        let scale = panel::scale(w, h);
        let screen = (w / scale, h / scale);
        let tokens = panel::tokens();
        let mut d = DrawList::new(raster::per_px(font));
        let top = match shown {
            Shown::Dialog(dialog) => {
                let labels: Vec<&str> = dialog.buttons.iter().map(|(label, _)| label.as_str()).collect();
                let keys: Vec<(&str, &str)> = dialog.keys.iter().map(|(key, what)| (*key, what.as_str())).collect();
                let rows = dialog.rows.len();
                let mut parts = vec![Part::Caption(&dialog.caption.0, &dialog.caption.1)];
                if !dialog.title.is_empty() {
                    parts.push(Part::Title(&dialog.title));
                }
                parts.extend(dialog.rows.iter().enumerate().map(|(i, row)| match row {
                    Row::Switch(label, on) => Part::Switch(label, *on, self.keys && dialog.focus == i),
                    Row::Field { text, hint, cursor } => Part::Field(text, hint, *cursor),
                }));
                parts.extend(dialog.lines.iter().map(|line| Part::Line(line, true)));
                parts.push(Part::Buttons(&labels, dialog.focus.checked_sub(rows).filter(|_| self.keys), dialog.gone));
                if self.keys {
                    parts.push(Part::Keys(&keys));
                }
                self.places = panel::draw(&mut d, &tokens, screen, &parts, !self.keys);
                // the glow above the edge is part of the strip
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
        Some(Some(raster::raster(&d, font, (0.0, top), (screen.0, screen.1 - top), scale)))
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
        let Some(index) = self.places.iter().position(|at| at.has(lx, ly)) else { return false };
        self.keys = false;
        flashed.press(index, fetched);
        true
    }

    /// A key, by the name the web gives it (`KeyboardEvent.key`: `ArrowLeft`, `Enter`, `Tab`, `Backspace`, a character
    /// …; the native deck names its keys the same). While a dialog shows, the keys are its own; a character goes into
    /// the field being edited; with no dialog, `f` opens "please don't flash" for who may set it. true = the key was the
    /// deck's UI's.
    pub fn key(&mut self, name: &str, flashed: &mut Flashed, fetched: &mut Fetched) -> bool {
        let dialog = matches!(flashed.shown(), Some(Shown::Dialog(_)));
        let key = match name {
            "ArrowLeft" => Key::Left,
            "ArrowRight" => Key::Right,
            "ArrowUp" => Key::Up,
            "ArrowDown" | "Tab" => Key::Down,
            "Enter" => Key::Enter,
            "Escape" => Key::Escape,
            "Backspace" => Key::Backspace,
            "Delete" => Key::Delete,
            "Home" => Key::Home,
            "End" => Key::End,
            _ if name.chars().count() == 1 && flashed.text(name) => return true,
            "f" | "F" if !dialog && flashed.open_keep() => {
                self.keys = true;
                return true;
            }
            _ => return false,
        };
        if !dialog {
            return false;
        }
        // the first key only shows where the keys are – but in a field every key is the text's
        if self.keys || flashed.editing().is_some() {
            flashed.key(key, fetched);
        }
        self.keys = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::FontRef;
    use protocol::Flash;

    fn font() -> FontRef<'static> {
        FontRef::try_from_slice(include_bytes!("../../outlet/assets/IBMPlexMono-Bold-latin.ttf")).expect("IBM Plex Mono")
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
        assert_eq!(face.paint(f.shown().as_ref(), SCREEN, &font), None, "nothing shows and nothing showed");
        f.flash(&flash("f1", 4), "", "no token", &mut fetched);
        // 800 × 600 is two device pixels per logical one: the strip is as wide as the screen
        let strip = face.paint(f.shown().as_ref(), SCREEN, &font).flatten().expect("the suggestion");
        assert!(strip.w == 800 && strip.h > 200 && strip.h < 600, "{} × {}", strip.w, strip.h);
        assert_eq!(face.paint(f.shown().as_ref(), SCREEN, &font), None, "nothing changed");
        // outside the buttons: not the dialog's; on the right half of the buttons' row: reject
        assert!(!face.tap(10.0, 10.0, &mut f, &mut fetched));
        let reject = face.places[1];
        assert!(face.tap((reject.x + 5.0) * 2.0, (reject.y + 5.0) * 2.0, &mut f, &mut fetched));
        assert!(f.reports().iter().any(|r| !r.accepted && r.why.as_deref() == Some("rejected at the deck")));
        // the toast: a small box at the bottom left, gone on a tap
        assert!(matches!(f.shown(), Some(Shown::Toast(..))));
        let toast = face.paint(f.shown().as_ref(), SCREEN, &font).flatten().expect("the toast");
        assert!(toast.h < 140 && toast.at(799, toast.h / 2)[3] == 0 && toast.at(40, toast.h / 2)[3] == 255, "transparent beside the box");
        let at = face.places[0];
        assert!(face.tap((at.x + 5.0) * 2.0, (at.y + 5.0) * 2.0, &mut f, &mut fetched));
        assert_eq!((f.shown(), face.paint(f.shown().as_ref(), SCREEN, &font)), (None, Some(None)));
    }

    #[test]
    fn please_dont_flash_is_opened_typed_and_saved_with_keys() {
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
        assert_eq!((d.rows.len(), d.focus, f.editing()), (2, 0, None));
        // Enter flips the switch, Tab goes to the reason, characters are typed, the keys of the text work
        for key in ["Enter", "Tab", "P", "r", "o", "x", "Backspace", "b", "e", "Home", "Delete", "p"] {
            assert!(face.key(key, &mut f, &mut fetched), "{key}");
        }
        assert_eq!(f.editing(), Some("probe"));
        assert!(face.paint(f.shown().as_ref(), SCREEN, &font).flatten().is_some());
        // a platform's own input hands the whole text
        assert!(f.set_field("Probe läuft", 11));
        assert_eq!(f.shown().and_then(|s| if let Shown::Dialog(d) = s { d.rows.get(1).cloned() } else { None }), Some(Row::Field { text: "Probe läuft".into(), hint: "warum".into(), cursor: Some(11) }));
        // Enter in the field saves: the conductor hears it at once, the person reads a toast
        assert!(face.key("Enter", &mut f, &mut fetched));
        assert_eq!(f.deck.keep(), Some("Probe läuft"));
        assert_eq!(f.reports().last().and_then(|r| r.keep.clone()).as_deref(), Some("Probe läuft"));
        assert!(matches!(f.shown(), Some(Shown::Toast(text, _)) if text == "Gespeichert: bitte nicht flashen (Probe läuft)"));
        // a deck that asks not to be flashed is not asked about a suggestion: it refuses by itself
        f.flash(&flash("f2", 4), "", "no token", &mut fetched);
        assert!(f.reports().iter().any(|r| !r.accepted) && !matches!(f.shown(), Some(Shown::Dialog(_))));
        // opened again it shows what stands; Escape leaves it
        assert!(face.key("f", &mut f, &mut fetched));
        assert!(matches!(f.shown(), Some(Shown::Dialog(d)) if d.rows[0] == Row::Switch("Bitte nicht flashen".into(), true)));
        assert!(face.key("Escape", &mut f, &mut fetched));
        assert_eq!(f.deck.keep(), Some("Probe läuft"));
    }
}
