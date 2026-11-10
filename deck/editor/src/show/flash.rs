//! Flash: choose the devices, then confirm (#471; the owner's draft 7 "Flash: choose the decks, then confirm"). Flash opens
//! the list of every device of the tree in the stage's place, all chosen but the decks that ask not to be flashed; the quick
//! filters, the typed filter and a hand set the ticks; "Flash …" opens the prompt, and only its confirm sends. The list, the
//! filters, the prompt's words and what the flash names are deck-show's choose.rs; this file lays them out and hands the
//! presses and the typed filter in (RULES A1, A2).
use super::perform::button;
use super::{now, Hit, Host, Sheet};
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;
use deck_show::choose::{Choice, Tone};
use deck_show::Lock;
use winit::keyboard::{Key, NamedKey};

/// The flash list while it is open.
pub struct Flashing {
    pub choice: Choice,
    lock: Lock,
    /// the prompt is up
    pub asking: bool,
    /// the typed filter's text, and whether keys go to it
    pub typed: String,
    pub typing: bool,
}

impl Host {
    /// Flash pressed: the tree resolved, the list opens (nothing is sent yet).
    pub(super) fn open_flash(&mut self) {
        match self.resolve() {
            Ok(lock) => {
                let choice = Choice::open(&self.sub, &lock);
                self.status = format!("{} of {} devices chosen", choice.count(), choice.lines.len());
                self.flashing = Some(Flashing { choice, lock, asking: false, typed: String::new(), typing: false });
                self.sheet = Some(Sheet::Flash);
            }
            Err(e) => self.status = e,
        }
    }

    pub(super) fn flash_press(&mut self, hit: Hit) {
        let Some(f) = self.flashing.as_mut() else { return };
        match hit {
            Hit::FlashTick(i) => f.choice.toggle(i),
            Hit::FlashQuick(i) => {
                if let Some((q, _)) = f.choice.quicks().get(i).cloned() {
                    f.choice.quick(&q);
                }
            }
            Hit::FlashType => f.typing = true,
            Hit::FlashLock => f.choice.locked = !f.choice.locked,
            Hit::FlashAsk => f.asking = true,
            Hit::FlashCancel if f.asking => f.asking = false,
            Hit::FlashCancel => {
                self.flashing = None;
                self.sheet = None;
            }
            Hit::FlashConfirm => {
                if !f.choice.prompt().can {
                    return;
                }
                let pick = f.choice.pick(None);
                let lock = f.lock.clone();
                let out = self.sub.flash_to(&lock, &pick, now());
                self.status = format!("flashed {} – {} left out", lock.shows.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "), pick.skip.len());
                self.send(out);
                self.flashing = None;
                self.sheet = None;
            }
            _ => {}
        }
    }

    /// A key while the typed filter has the keys: true = taken. Enter applies it (the core reads it), Escape leaves it.
    pub fn type_key(&mut self, key: &Key) -> bool {
        let Some(f) = self.flashing.as_mut().filter(|f| f.typing) else { return false };
        match key {
            Key::Character(c) => f.typed.push_str(c),
            Key::Named(NamedKey::Space) => f.typed.push(' '),
            Key::Named(NamedKey::Backspace) => {
                f.typed.pop();
            }
            Key::Named(NamedKey::Enter) => {
                f.typing = false;
                let text = f.typed.clone();
                self.status = match f.choice.filter(&text) {
                    Ok(n) => format!("{n} devices match"),
                    Err(e) => e,
                };
            }
            Key::Named(NamedKey::Escape) => f.typing = false,
            _ => {}
        }
        true
    }

    /// A check run: the typed filter applied as if typed and entered.
    pub fn script_filter(&mut self, text: &str) {
        if let Some(f) = self.flashing.as_mut() {
            f.typed = text.to_string();
            f.typing = true;
        }
        self.type_key(&Key::Named(NamedKey::Enter));
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let Some(f) = host.flashing.as_ref() else { return };
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.row);
    let c = &f.choice;
    let mut hits = vec![];
    let mut y = a.y + pad;
    d.text(a.x + pad, y, small, &format!("FLASH – {} OF {} DEVICES CHOSEN", c.count(), c.lines.len()), t.color.muted);
    y += small * 1.8;
    // the quick filters, then the typed one
    let quicks = c.quicks();
    let names: Vec<&str> = quicks.iter().map(|(_, w)| w.as_str()).collect();
    for (i, r) in controls::pills(d, t, a.x + pad, y, row - 4.0, &names, usize::MAX).into_iter().enumerate() {
        hits.push((r, Hit::FlashQuick(i)));
    }
    y += row + 2.0;
    let field = Rect { x: a.x + pad, y, w: a.w - 2.0 * pad, h: row - 4.0 };
    d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
    if f.typing {
        d.outline(field.x, field.y, field.w, field.h, (t.size.radius, 1.0), t.color.accent);
    }
    let (text, color) = match (f.typed.is_empty(), f.typing) {
        (true, false) => (deck_show::choose::FILTER_HINT.to_string(), t.color.muted),
        (_, true) => (format!("{}|", f.typed), t.color.text),
        (false, false) => (f.typed.clone(), t.color.text),
    };
    d.text(field.x + 6.0, field.y + 4.0, font, &fit(d, &text, font, field.w - 12.0), color);
    hits.push((field, Hit::FlashType));
    y += row;
    if let Some(e) = &c.error {
        d.text(a.x + pad, y, small, &fit(d, e, small, a.w - 2.0 * pad), t.color.danger);
        y += small * 1.6;
    }
    d.rect(a.x, y, a.w, 1.0, t.color.line);
    y += 4.0;
    // the list
    let lh = font * 2.6;
    let foot = a.y + a.h - row - 2.0 * pad;
    if c.lines.is_empty() {
        d.text(a.x + pad, y + 4.0, font, "No device has answered yet: decks that join later get the flash too.", t.color.muted);
    }
    for (i, l) in c.lines.iter().enumerate() {
        if y + lh > foot {
            d.text(a.x + pad, y + 2.0, small, &format!("{} more …", c.lines.len() - i), t.color.muted);
            break;
        }
        let whole = Rect { x: a.x, y, w: a.w, h: lh };
        if whole.has(pointer.0, pointer.1) {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.hover);
        }
        let indent = if l.path.contains('/') { 2.0 * pad } else { 0.0 };
        let (bx, by, bs) = (a.x + pad + indent, y + (lh - 14.0) * 0.5, 14.0);
        if c.is_ticked(i) {
            d.round_rect(bx, by, bs, bs, 3.0, t.color.accent);
        } else {
            d.outline(bx, by, bs, bs, (3.0, 1.0), t.color.muted);
        }
        let tx = bx + bs + pad;
        let tw = a.x + a.w - tx - pad;
        d.text(tx, y + 3.0, font, &fit(d, &format!("{} · {}", l.head(), l.kind), font, tw), if l.online { t.color.text } else { t.color.muted });
        let state_color = if l.keep.is_some() || l.unsaved { t.color.warn } else { t.color.muted };
        d.text(tx, y + 3.0 + font * 1.3, small, &fit(d, &l.state(), small, tw), state_color);
        d.rect(a.x, y + lh - 1.0, a.w, 1.0, t.color.line);
        hits.push((whole, Hit::FlashTick(i)));
        y += lh;
    }
    // the foot: the lock, cancel, on to the prompt
    let bw = ((a.w - 4.0 * pad) / 3.0).min(220.0);
    let lock = Rect { x: a.x + pad, y: foot + pad, w: bw, h: row };
    let cancel = Rect { x: a.x + a.w - 2.0 * (bw + pad), y: foot + pad, w: bw, h: row };
    let ask = Rect { x: a.x + a.w - bw - pad, y: foot + pad, w: bw, h: row };
    let lock_word = if c.locked { "Locked" } else { "Not locked" };
    button(d, t, lock, lock_word, if c.locked { t.color.selected } else { t.color.control }, t.color.text);
    button(d, t, cancel, "Cancel", t.color.control, t.color.text);
    button(d, t, ask, &format!("Flash {} …", c.count()), t.color.accent, t.color.surface);
    hits.extend([(lock, Hit::FlashLock), (cancel, Hit::FlashCancel), (ask, Hit::FlashAsk)]);
    if f.asking {
        // the prompt over the list: its presses only
        hits.clear();
        prompt(d, t, c, a, &mut hits);
    }
    host.hits.extend(hits);
}

fn prompt(d: &mut DrawList, t: &Tokens, c: &Choice, a: Rect, hits: &mut Vec<(Rect, Hit)>) {
    let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
    let p = c.prompt();
    d.rect(a.x, a.y, a.w, a.h, [t.color.stage[0], t.color.stage[1], t.color.stage[2], 0.7]);
    let w = (a.w - 4.0 * pad).min(640.0);
    let lh = font * 1.5;
    let h = pad * 3.0 + font * 1.8 + p.lines.len() as f32 * lh + row + pad;
    let (x, y) = (a.x + (a.w - w) * 0.5, a.y + ((a.h - h) * 0.4).max(pad));
    d.round_rect(x, y, w, h, t.size.radius, t.color.surface);
    d.outline(x, y, w, h, (t.size.radius, 1.0), if p.warn { t.color.warn } else { t.color.line });
    d.text(x + pad, y + pad, font * 1.15, &fit(d, &p.title, font * 1.15, w - 2.0 * pad), t.color.text);
    let mut ly = y + pad + font * 1.8 + pad * 0.5;
    for (line, tone) in &p.lines {
        let color = match tone {
            Tone::Warn => t.color.warn,
            Tone::Plain => t.color.text,
            Tone::Muted => t.color.muted,
        };
        if *tone == Tone::Warn {
            d.disc(x + pad + 3.0, ly + font * 0.62, 3.5, t.color.warn);
        }
        d.text(x + pad + 12.0, ly, font, &fit(d, line, font, w - 2.0 * pad - 12.0), color);
        ly += lh;
    }
    let bw = 160.0_f32.min((w - 3.0 * pad) / 2.0);
    let by = y + h - row - pad;
    let cancel = Rect { x: x + w - 2.0 * (bw + pad), y: by, w: bw, h: row };
    let confirm = Rect { x: x + w - bw - pad, y: by, w: bw, h: row };
    button(d, t, cancel, "Back", t.color.control, t.color.text);
    if p.can {
        button(d, t, confirm, &p.confirm, if p.warn { t.color.danger } else { t.color.accent }, t.color.surface);
        hits.push((confirm, Hit::FlashConfirm));
    }
    hits.push((cancel, Hit::FlashCancel));
}
