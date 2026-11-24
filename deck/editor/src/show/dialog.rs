//! The Show's dialogs on the overlay level (RULES H15): Flash (flash.rs, RULES H18), a new scene or a recast
//! (new_scene.rs), a new show (toons.rs) – one-off tasks that end in a confirm or a save. Each is a box of the kit's
//! overlay over the blurred workspace (main.rs lays the ground under it): its title and × at the head, its body, its foot
//! with one primary action. While one is open only its places take a press; Esc and a press outside close it.
use super::{Hit, Host, Sheet};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};

impl Sheet {
    /// it opens as a dialog on the overlay level, not in the sheet panel
    pub fn dialog(self) -> bool {
        matches!(self, Sheet::Flash | Sheet::Scene | Sheet::NewShow)
    }
}

impl Host {
    /// a dialog is open over the Show
    pub fn dialog(&self) -> bool {
        self.sheet.is_some_and(Sheet::dialog)
    }

    pub(super) fn close_dialog(&mut self) {
        if self.dialog() {
            self.sheet = None;
        }
        self.flashing = None;
        self.typing = None;
    }

    /// Esc while a dialog is open: a field being typed stops, the confirm steps back, else the dialog closes
    pub fn dialog_escape(&mut self) {
        if self.typing.take().is_some() {
            return;
        }
        match self.flashing.as_mut() {
            Some(f) if f.typing => f.typing = false,
            Some(f) if f.step == super::flash::Step::Ask => f.step = super::flash::Step::List,
            _ => self.close_dialog(),
        }
    }

    /// the open dialog's title
    pub fn dialog_title(&self) -> String {
        match self.sheet {
            Some(Sheet::Flash) => format!("Flash – {}", self.sub.conductor().show().id),
            Some(Sheet::Scene) if self.compose.doc.is_some() => format!("Scene scene:{}", self.compose.id),
            Some(Sheet::Scene) => "New scene".into(),
            Some(Sheet::NewShow) => "New show".into(),
            _ => String::new(),
        }
    }
}

/// the box's size for the dialog open, in a window `room` big
fn size(t: &Tokens, sheet: Sheet, room: (f32, f32)) -> (f32, f32) {
    let (w, h) = match sheet {
        Sheet::Scene => (t.size.rem(40.0), t.size.rem(37.0)),
        Sheet::NewShow => (t.size.rem(34.0), t.size.rem(26.0)),
        _ => (t.size.rem(34.0), t.size.rem(30.0)),
    };
    let gap = t.size.gap * 2.0;
    (w.min(room.0 - 2.0 * gap), h.min(room.1 - 2.0 * gap))
}

/// The open dialog over the window `room` big: its box, its head, its body. The Show's places behind it take no press.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, room: (f32, f32), pointer: (f32, f32)) {
    let Some(sheet) = host.sheet.filter(|s| s.dialog()) else { return };
    host.hits.clear();
    host.hits.push((Rect { x: 0.0, y: 0.0, w: room.0, h: room.1 }, Hit::DialogClose));
    let (w, h) = size(t, sheet, room);
    let r = Rect { x: (room.0 - w) * 0.5, y: ((room.1 - h) * 0.4).max(t.size.gap), w, h };
    host.hits.push((r, Hit::Ground));
    let s = t.kit("overlay", "box");
    let radius = t.size.radius * 2.0;
    d.round_rect(r.x, r.y, r.w, r.h, radius, s.ground.unwrap_or(t.color.popover()));
    if let Some(b) = s.border {
        d.outline(r.x, r.y, r.w, r.h, (radius, t.size.edge), b);
    }
    let halo = d.halo_state();
    d.halo(t.color.ring());
    let (pad, font) = (t.size.pad, t.size.font);
    let head = t.size.row;
    let title = host.dialog_title();
    let close = Rect { x: r.x + r.w - pad * 0.5 - head * 0.8, y: r.y + pad * 0.5 + head * 0.1, w: head * 0.8, h: head * 0.8 };
    d.sans(r.x + pad * 1.5, r.y + pad * 0.5 + (head - font * 1.25) * 0.5, font, &controls::fit_sans(d, font, &title, close.x - r.x - pad * 2.0), t.color.text);
    controls::icon_button(d, t, close, "close", t.color.muted, pointer);
    host.hits.push((close, Hit::DialogClose));
    let body = Rect { x: r.x + pad * 1.5, y: r.y + pad * 0.5 + head + pad * 0.5, w: r.w - pad * 3.0, h: r.h - head - pad * 2.5 };
    match sheet {
        Sheet::Flash => super::flash::draw(d, t, host, body, pointer),
        Sheet::Scene => super::new_scene::draw(d, t, host, body, pointer),
        Sheet::NewShow => super::toons::new_sheet(d, t, host, body, pointer),
        _ => {}
    }
    d.restore_halo(halo);
}

/// what a place of a dialog means (the Info view and the hover, H14)
pub fn help(host: &Host, x: f32, y: f32) -> Option<(String, String)> {
    if !host.dialog() {
        return None;
    }
    let hit = host.hit(x, y)?;
    super::flash::help(host, hit).or_else(|| match hit {
        Hit::DialogClose => Some(("Close".into(), "Closes it; nothing more is sent or made.".into())),
        Hit::Compose(c) => super::new_scene::help(c),
        _ => None,
    })
}

/// A field of the kit in `r`: its value (or the text being typed, with the caret), else its hint muted; Find's search icon
/// first when `search`; its place.
pub fn input(d: &mut DrawList, t: &Tokens, r: Rect, (value, typing): (&str, Option<&str>), (hint, search): (&str, bool), pointer: (f32, f32)) -> Rect {
    let state = if typing.is_some() {
        "on"
    } else if r.has(pointer.0, pointer.1) {
        "hover"
    } else {
        "rest"
    };
    controls::field(d, t, r, "", ("field", state), false);
    let (pad, px) = (t.size.pad, t.size.font);
    let text = typing.map_or_else(|| value.to_string(), |s| format!("{s}|"));
    let mut x = r.x + pad;
    if search {
        let s = px;
        d.icon(x, r.y + (r.h - s) * 0.5, s, "search", t.color.muted);
        x += s + pad * 0.6;
    }
    let (shown, ink) = if text.is_empty() { (hint, t.color.muted) } else { (text.as_str(), t.color.text) };
    d.sans(x, r.y + (r.h - px * 1.25) * 0.5, px, &controls::fit_sans(d, px, shown, r.x + r.w - pad - x), ink);
    r
}

/// a label with a field of the kit beside it, the label's column `col` wide, the field's hint; the field's place
pub fn labelled(d: &mut DrawList, t: &Tokens, (x, y, w, col): (f32, f32, f32, f32), (label, hint): (&str, &str), value: (&str, Option<&str>), pointer: (f32, f32)) -> Rect {
    let h = t.size.row - 4.0;
    controls::label(d, t, x, y + (h - t.size.label * 1.25) * 0.5, label, t.color.muted);
    input(d, t, Rect { x: x + col, y, w: w - col, h }, value, (hint, false), pointer)
}

/// `n of m chosen` as a label, All · None quiet at its right; their places
pub fn count_head(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), words: &str, pointer: (f32, f32)) -> (Rect, Rect) {
    let h = t.size.row - 6.0;
    controls::label(d, t, x, y + (h - t.size.label * 1.25) * 0.5, words, t.color.muted);
    let px = t.size.font;
    let bw = |s: &str| d.sans_width(px, s) + t.size.pad * 3.0;
    let none_x = x + w - bw("None");
    let all_x = none_x - bw("All") - 2.0;
    let state = |r: Rect| if r.has(pointer.0, pointer.1) { "hover" } else { "rest" };
    let all = Rect { x: all_x, y, w: bw("All"), h };
    let none = Rect { x: none_x, y, w: bw("None"), h };
    controls::button(d, t, (all.x, y, h), "All", "button_quiet", state(all));
    controls::button(d, t, (none.x, y, h), "None", "button_quiet", state(none));
    (all, none)
}

/// one line of a ticked list
pub struct Tick {
    pub label: String,
    pub detail: String,
    pub on: bool,
    /// the detail warns (a request, unsaved configuration …)
    pub warns: bool,
    /// it sits under the line before it
    pub indent: bool,
    /// a box to tick (else a press picks the line)
    pub tick: bool,
}

/// The ticked list in `a`: a box ticked or not, the label, the detail at the right; "n more …" where it ends. The lines'
/// places.
pub fn ticks(d: &mut DrawList, t: &Tokens, a: Rect, lines: &[Tick], pointer: (f32, f32)) -> Vec<Rect> {
    let (pad, px, small) = (t.size.pad, t.size.font, t.size.small);
    let lh = t.size.tree_row;
    let mut out = Vec::with_capacity(lines.len());
    let mut y = a.y;
    for (i, l) in lines.iter().enumerate() {
        if y + lh > a.y + a.h {
            d.sans(a.x, y + 2.0, small, &format!("{} more …", lines.len() - i), t.color.muted);
            break;
        }
        let whole = Rect { x: a.x - pad * 0.5, y, w: a.w + pad, h: lh };
        let state = if whole.has(pointer.0, pointer.1) { "hover" } else { "rest" };
        if let Some(g) = t.kit("list_row", state).ground {
            d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, g);
        }
        let s = px * 1.05;
        let bx = a.x + if l.indent { pad * 1.5 } else { 0.0 };
        let by = y + (lh - s) * 0.5;
        let tx = if l.tick {
            tick_box(d, t, Rect { x: bx, y: by, w: s, h: s }, l.on);
            bx + s + pad * 0.8
        } else {
            bx
        };
        let detail_w = if l.detail.is_empty() { 0.0 } else { d.sans_width(small, &l.detail).min(a.w * 0.5) };
        d.sans(tx, y + (lh - px * 1.25) * 0.5, px, &controls::fit_sans(d, px, &l.label, a.x + a.w - detail_w - pad - tx), t.color.text);
        if detail_w > 0.0 {
            let ink = if l.warns { t.color.warn } else { t.color.muted };
            let detail = controls::fit_sans(d, small, &l.detail, detail_w);
            d.sans(a.x + a.w - d.sans_width(small, &detail), y + (lh - small * 1.25) * 0.5, small, &detail, ink);
        }
        out.push(whole);
        y += lh;
    }
    out
}

/// a tick box: on is the pressed fill with its tick, off the line
fn tick_box(d: &mut DrawList, t: &Tokens, r: Rect, on: bool) {
    let radius = r.h * 0.22;
    if on {
        d.round_rect(r.x, r.y, r.w, r.h, radius, t.color.pressed);
        let ink = t.color.pressed_text;
        let th = (r.h * 0.13).max(1.2);
        let (a, b, c) = ((r.x + r.w * 0.24, r.y + r.h * 0.52), (r.x + r.w * 0.43, r.y + r.h * 0.7), (r.x + r.w * 0.78, r.y + r.h * 0.3));
        d.segment(a, b, th, ink);
        d.segment(b, c, th, ink);
    } else {
        d.outline(r.x, r.y, r.w, r.h, (radius, t.size.edge), t.color.line);
    }
}

/// Segments of the kit side by side, one on (the pressed fill); their places
pub fn segments(d: &mut DrawList, t: &Tokens, (x, y, h): (f32, f32, f32), names: &[&str], on: usize, pointer: (f32, f32)) -> Vec<Rect> {
    let px = t.size.font;
    let widths: Vec<f32> = names.iter().map(|n| d.sans_width(px, n) + t.size.pad * 2.0).collect();
    let whole = Rect { x, y, w: widths.iter().sum(), h };
    let rest = t.kit("segments", "rest");
    if let Some(g) = rest.ground {
        d.round_rect(whole.x, whole.y, whole.w, whole.h, t.size.radius, g);
    }
    if let Some(b) = rest.border {
        d.outline(whole.x, whole.y, whole.w, whole.h, (t.size.radius, t.size.edge), b);
    }
    let mut cx = x;
    let mut out = Vec::with_capacity(names.len());
    for (i, (name, w)) in names.iter().zip(&widths).enumerate() {
        let r = Rect { x: cx, y, w: *w, h };
        let state = if i == on {
            "on"
        } else if r.has(pointer.0, pointer.1) {
            "hover"
        } else {
            "rest"
        };
        let s = t.kit("segments", state);
        if state != "rest" {
            if let Some(g) = s.ground {
                d.round_rect(r.x + 2.0, r.y + 2.0, r.w - 4.0, r.h - 4.0, (t.size.radius - 2.0).max(1.0), g);
            }
        }
        d.sans(r.x + t.size.pad, y + (h - px * 1.25) * 0.5, px, name, s.text.unwrap_or(t.color.text));
        out.push(r);
        cx += w;
    }
    out
}

/// what a dialog's foot holds: plain buttons at the left, plain ones at the right, then the one primary action (its words,
/// its press, whether it can be done now)
pub struct Foot<'a> {
    pub left: &'a [(&'a str, Hit)],
    pub right: &'a [(&'a str, Hit)],
    pub primary: Option<(&'a str, Hit, bool)>,
}

/// The foot in `r`: the buttons and their presses (a primary that cannot be done now is drawn faded and takes none)
pub fn foot(d: &mut DrawList, t: &Tokens, r: Rect, f: &Foot, pointer: (f32, f32)) -> Vec<(Rect, Hit)> {
    let (pad, px) = (t.size.pad, t.size.font);
    let h = r.h;
    let bw = |d: &DrawList, s: &str| d.sans_width(px, s) + pad * 3.0;
    let state = |b: Rect| if b.has(pointer.0, pointer.1) { "hover" } else { "rest" };
    let mut out = Vec::new();
    let mut x = r.x;
    for &(words, hit) in f.left {
        let b = Rect { x, y: r.y, w: bw(d, words), h };
        controls::button(d, t, (b.x, b.y, h), words, "button_plain", state(b));
        out.push((b, hit));
        x += b.w + pad * 0.5;
    }
    let mut x = r.x + r.w;
    if let Some((words, hit, can)) = f.primary {
        let b = Rect { x: x - bw(d, words), y: r.y, w: bw(d, words), h };
        controls::primary_in(d, t, b, words, false, if !can { "disabled" } else { state(b) });
        if can {
            out.push((b, hit));
        }
        x = b.x - pad * 0.5;
    }
    for &(words, hit) in f.right.iter().rev() {
        let b = Rect { x: x - bw(d, words), y: r.y, w: bw(d, words), h };
        controls::button(d, t, (b.x, b.y, h), words, "button_plain", state(b));
        out.push((b, hit));
        x = b.x - pad * 0.5;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::flash::Step;
    use super::super::rows::tests::{host_with, press};
    use super::super::{Hit, Sheet};

    const TOP: &str = r#"{ "id": "top", "version": 3, "toons": ["catpope"], "outlets": [{ "id": "left" }, { "id": "right" }], "rows": [] }"#;

    /// H18: the bar's Flash opens the dialog: every device ticked, All · None, "Flash N", the confirm in the same box, then
    /// the report there; Esc steps back from the confirm, a press outside closes it
    #[test]
    fn flash_is_one_dialog_list_confirm_report() {
        let mut host = host_with(TOP);
        host.press(Hit::Flash);
        assert_eq!(host.sheet, Some(Sheet::Flash));
        assert!(host.dialog());
        assert_eq!(host.dialog_title(), "Flash – top");
        press(&mut host, |h| *h == Hit::FlashNone);
        assert_eq!(host.flashing.as_ref().unwrap().choice.count(), 0);
        press(&mut host, |h| *h == Hit::FlashAll);
        press(&mut host, |h| *h == Hit::FlashCancel);
        assert!(!host.dialog(), "Cancel closes it");
        host.press(Hit::Flash);
        host.flashing.as_mut().unwrap().step = Step::Ask;
        press(&mut host, |h| *h == Hit::FlashForce(true));
        assert!(host.flashing.as_ref().unwrap().choice.forced);
        host.dialog_escape();
        assert_eq!(host.flashing.as_ref().unwrap().step, Step::List, "Esc steps back from the confirm");
        host.dialog_escape();
        assert!(!host.dialog());
        host.press(Hit::Flash);
        // a press outside the box closes it; the Show's panels behind take none
        press(&mut host, |h| *h == Hit::DialogClose);
        assert!(!host.dialog());
    }

    /// New show is a dialog on the overlay level, not the sheet panel's
    #[test]
    fn new_show_and_scene_are_dialogs() {
        let mut host = host_with(TOP);
        host.press(Hit::Sheet(Sheet::NewShow));
        assert!(host.dialog());
        press(&mut host, |h| *h == Hit::NewName);
        host.press(Hit::DialogClose);
        host.press(Hit::Compose(super::super::new_scene::ComposeHit::Open));
        assert!(host.dialog());
        assert_eq!(host.dialog_title(), "New scene");
        press(&mut host, |h| *h == Hit::Compose(super::super::new_scene::ComposeHit::Close));
        assert!(!host.dialog());
    }
}
