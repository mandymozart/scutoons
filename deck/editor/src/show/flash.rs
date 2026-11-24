//! Flash: the bar's box icon opens it as a dialog on the overlay level (RULES H18, the same act as the creator's Publish):
//! every device of the tree, ticked but the decks that ask not to be flashed; Find, All · None; one primary button with
//! the count; then the confirm in the same box, a suggestion or forced, locked or not; then the report there. The list, its
//! words, the filter and what the flash names are deck-show's choose.rs; this file lays them out and hands the presses
//! and the typed filter in (RULES A1, A2).
use super::dialog::{self, Foot};
use super::live::button;
use super::{now, Hit, Host, Sheet};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::choose::{Choice, Prompt, Tone};
use deck_show::Lock;
use winit::keyboard::{Key, NamedKey};

/// where the flash dialog is
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// the devices to tick
    List,
    /// the confirm: only its Flash sends
    Ask,
    /// what was sent, and the decks' answers as they come
    Report(String),
}

/// The flash dialog while it is open.
pub struct Flashing {
    pub choice: Choice,
    lock: Lock,
    pub step: Step,
    /// the typed filter's text, and whether keys go to it
    pub typed: String,
    pub typing: bool,
}

impl Host {
    /// The bar's Flash: the tree resolved, the dialog opens (nothing is sent yet).
    pub(super) fn open_flash(&mut self) {
        match self.resolve() {
            Ok(lock) => {
                let choice = Choice::open(&self.sub, &lock);
                self.flashing = Some(Flashing { choice, lock, step: Step::List, typed: String::new(), typing: false });
                self.sheet = Some(Sheet::Flash);
            }
            Err(e) => self.refuse(e),
        }
    }

    /// Flash's dot: a device of the tree here is not on the version this conductor holds (deck-show `tally`)
    pub fn flash_dot(&self) -> deck_show::choose::Tally {
        deck_show::choose::tally(&self.sub)
    }

    pub(super) fn flash_press(&mut self, hit: Hit) {
        let Some(f) = self.flashing.as_mut() else { return };
        f.typing = f.typing && hit == Hit::FlashType;
        match hit {
            Hit::FlashTick(i) => f.choice.toggle(i),
            Hit::FlashAll => f.choice.quick(&deck_show::choose::Quick::All),
            Hit::FlashNone => f.choice.quick(&deck_show::choose::Quick::None),
            Hit::FlashType => f.typing = true,
            Hit::FlashLock(on) => f.choice.locked = on,
            Hit::FlashForce(on) => f.choice.forced = on,
            Hit::FlashAsk if f.choice.count() > 0 => f.step = Step::Ask,
            Hit::FlashCancel if f.step == Step::Ask => f.step = Step::List,
            Hit::FlashCancel => self.close_dialog(),
            Hit::FlashConfirm => {
                if !f.choice.prompt().can {
                    return;
                }
                let pick = f.choice.pick(None);
                let lock = f.lock.clone();
                let out = self.sub.flash_to(&lock, &pick, now());
                let said = format!("flashed {} – {} left out", lock.shows.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "), pick.skip.len());
                self.status.clone_from(&said);
                self.send(out);
                if let Some(f) = self.flashing.as_mut() {
                    f.step = Step::Report(said);
                }
            }
            _ => {}
        }
    }

    /// A key while the typed filter has the keys: true = taken. Enter applies it (the core reads it), Escape leaves it.
    pub fn type_key(&mut self, key: &Key) -> bool {
        // #551: the right click's typeahead has the keys while it is open
        if self.context_key(key) {
            return true;
        }
        // #478: a role's name being typed
        if self.type_field(key) {
            return true;
        }
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

    /// A check run (`--devices`): decks answer as the relay would hand their flash reports on, each on an outlet and a
    /// version of the show; the ones marked offline are not in the relay's presence.
    pub fn script_devices(&mut self, devices: &[Device]) {
        let id = self.sub.conductor().show().id.clone();
        let mut links = serde_json::Map::new();
        for dev in devices {
            let mut r = deck_show::flash::report(&deck_show::Pinned { id: id.clone(), version: dev.version }, true, None);
            (r.outlet, r.device) = (Some(dev.outlet.clone()), Some("native".into()));
            let out = self.sub.on_wire(&protocol::Envelope::new(dev.id.clone(), protocol::Message::FlashReport(r)), now());
            self.send(out);
            if dev.online {
                links.insert(dev.id.clone(), serde_json::json!({ "hz": 30 }));
            }
        }
        let presence = serde_json::json!({ "groups": {}, "ready": [], "links": links, "seats": {} });
        if let Ok(p) = serde_json::from_value(presence) {
            let out = self.sub.on_wire(&protocol::Envelope::new("relay", protocol::Message::Presence(p)), now());
            self.send(out);
        }
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

/// a deck a check run makes answer: its client id, its outlet, the show's version it runs, whether it is here
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub outlet: String,
    pub version: u32,
    pub online: bool,
}

/// `ID=OUTLET@VERSION[~],…` (`~`: not here)
pub fn parse_devices(v: &str) -> Option<Vec<Device>> {
    v.split(',')
        .map(|d| {
            let (id, rest) = d.trim().split_once('=')?;
            let (rest, online) = rest.strip_suffix('~').map_or((rest, true), |r| (r, false));
            let (outlet, version) = rest.split_once('@')?;
            Some(Device { id: id.into(), outlet: outlet.into(), version: version.parse().ok()?, online })
        })
        .collect()
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit_sans(d, px, text, w)
}

/// The flash dialog's body in `a`: the step it is at.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let Some(f) = host.flashing.as_ref() else { return };
    match f.step.clone() {
        Step::List => list(d, t, host, a, pointer),
        Step::Ask => ask(d, t, host, a, pointer),
        Step::Report(said) => report(d, t, host, a, &said, pointer),
    }
}

fn list(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let Some(f) = host.flashing.as_ref() else { return };
    let (pad, row) = (t.size.pad, t.size.row);
    let c = &f.choice;
    let mut hits = vec![];
    let mut y = a.y;
    let shown = if f.typing { Some(f.typed.as_str()) } else { None };
    let field = dialog::input(d, t, Rect { x: a.x, y, w: a.w, h: row - 4.0 }, (&c.filter, shown), ("Find", true), pointer);
    hits.push((field, Hit::FlashType));
    y += row + pad * 0.5;
    if let Some(e) = &c.error {
        d.sans(a.x, y, t.size.small, &fit(d, e, t.size.small, a.w), t.color.danger);
        y += t.size.small * 1.6;
    }
    let (all, none) = dialog::count_head(d, t, (a.x, y, a.w), &format!("{} of {} chosen", c.count(), c.lines.len()), pointer);
    hits.extend([(all, Hit::FlashAll), (none, Hit::FlashNone)]);
    y += row;
    let foot = a.y + a.h - row - pad;
    let lines: Vec<dialog::Tick> = c
        .lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let (word, warns) = l.word();
            dialog::Tick { label: l.head(), detail: word, on: c.is_ticked(i), warns, indent: l.path.contains('/'), tick: true }
        })
        .collect();
    if lines.is_empty() {
        d.sans(a.x, y + 4.0, t.size.font, "No device has answered yet: decks that join later get the flash too.", t.color.muted);
    }
    for (i, r) in dialog::ticks(d, t, Rect { x: a.x, y, w: a.w, h: foot - y - pad }, &lines, pointer).into_iter().enumerate() {
        hits.push((r, Hit::FlashTick(i)));
    }
    let n = c.count();
    let go = format!("Flash {n}");
    hits.extend(dialog::foot(d, t, Rect { x: a.x, y: foot, w: a.w, h: row }, &Foot { left: &[("Cancel", Hit::FlashCancel)], right: &[], primary: Some((&go, Hit::FlashAsk, n > 0)) }, pointer));
    host.hits.extend(hits);
}

fn ask(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let Some(f) = host.flashing.as_ref() else { return };
    let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
    let c = &f.choice;
    let p = c.prompt();
    let mut hits = vec![];
    let mut y = a.y;
    let px = font * 1.2;
    d.sans(a.x, y, px, &fit(d, &p.title, px, a.w), t.color.text);
    y += px * 1.25 + pad;
    let h = row - 4.0;
    let forced = dialog::segments(d, t, (a.x, y, h), &["A suggestion", "Forced"], usize::from(c.forced), pointer);
    hits.extend(forced.into_iter().enumerate().map(|(i, r)| (r, Hit::FlashForce(i == 1))));
    y += h + pad * 0.6;
    let locked = dialog::segments(d, t, (a.x, y, h), &["Not locked", "Locked"], usize::from(c.locked), pointer);
    hits.extend(locked.into_iter().enumerate().map(|(i, r)| (r, Hit::FlashLock(i == 1))));
    y += h + pad;
    // what each device loses or keeps (what a choice means is its hover text, H14)
    let foot = a.y + a.h - row - pad;
    for (line, tone) in c.says().0.iter().filter(|(_, tone)| *tone != Tone::Muted) {
        if y + font * 1.5 > foot {
            break;
        }
        let ink = if *tone == Tone::Warn { t.color.warn } else { t.color.text };
        if *tone == Tone::Warn {
            d.disc(a.x + 3.0, y + font * 0.62, 3.5, t.color.warn);
        }
        d.sans(a.x + 12.0, y, font, &fit(d, line, font, a.w - 12.0), ink);
        y += font * 1.5;
    }
    let confirm = p.confirm.clone();
    hits.extend(dialog::foot(d, t, Rect { x: a.x, y: foot, w: a.w, h: row }, &Foot { left: &[("Back", Hit::FlashCancel)], right: &[], primary: Some((&confirm, Hit::FlashConfirm, p.can)) }, pointer));
    host.hits.extend(hits);
}

fn report(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, said: &str, pointer: (f32, f32)) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.row);
    let mut y = a.y;
    d.sans(a.x, y, font * 1.2, &fit(d, said, font * 1.2, a.w), t.color.text);
    y += font * 1.5 + pad;
    // what each deck and sub conductor answered, as it comes (deck-show's words)
    let foot = a.y + a.h - row - pad;
    for r in deck_show::view::reports(&host.sub) {
        if y + font * 1.5 > foot {
            break;
        }
        d.disc(a.x + 4.0, y + font * 0.62, 3.5, if r.ok { t.color.go } else { t.color.warn });
        d.sans(a.x + 14.0, y, font, &fit(d, &format!("{} · {}", r.what, r.from), font, a.w - 14.0), t.color.text);
        y += font * 1.5;
        for l in std::iter::once(&r.line).chain(&r.lacks) {
            d.sans(a.x + 14.0, y, small, &fit(d, l, small, a.w - 14.0), t.color.muted);
            y += small * 1.6;
        }
    }
    let hits = dialog::foot(d, t, Rect { x: a.x, y: foot, w: a.w, h: row }, &Foot { left: &[], right: &[("Close", Hit::DialogClose)], primary: None }, pointer);
    host.hits.extend(hits);
}

/// a prompt of deck-show's (a role's remove, a kick, the History's revert) over `a`: its confirm and its back press `yes`, `no`
pub(crate) fn prompt<H: Copy>(d: &mut DrawList, t: &Tokens, p: &Prompt, a: Rect, hits: &mut Vec<(Rect, H)>, (yes, no): (H, H)) {
    let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
    d.rect(a.x, a.y, a.w, a.h, [t.color.stage[0], t.color.stage[1], t.color.stage[2], 0.7]);
    let w = (a.w - 4.0 * pad).min(640.0);
    let lh = font * 1.5;
    let h = pad * 3.0 + font * 1.8 + p.lines.len() as f32 * lh + row + pad;
    let (x, y) = (a.x + (a.w - w) * 0.5, a.y + ((a.h - h) * 0.4).max(pad));
    d.round_rect(x, y, w, h, t.size.radius, t.color.popover());
    d.outline(x, y, w, h, (t.size.radius, 1.0), if p.warn { t.color.warn } else { t.color.line });
    d.text(x + pad, y + pad, font * 1.15, &controls::fit(&p.title, ((w - 2.0 * pad) / d.advance(font * 1.15)).max(0.0) as usize), t.color.text);
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
        d.text(x + pad + 12.0, ly, font, &controls::fit(line, ((w - 2.0 * pad - 12.0) / d.advance(font)).max(0.0) as usize), color);
        ly += lh;
    }
    let bw = 160.0_f32.min((w - 3.0 * pad) / 2.0);
    let by = y + h - row - pad;
    let cancel = Rect { x: x + w - 2.0 * (bw + pad), y: by, w: bw, h: row };
    let confirm = Rect { x: x + w - bw - pad, y: by, w: bw, h: row };
    button(d, t, cancel, "Back", t.color.control, t.color.text);
    if p.can {
        if p.warn {
            button(d, t, confirm, &p.confirm, t.color.danger, t.color.bright);
        } else {
            controls::primary(d, t, confirm, &p.confirm, false);
        }
        hits.push((confirm, yes));
    }
    hits.push((cancel, no));
}

/// what a place of the flash dialog means (the Info view and the hover, H14)
pub fn help(host: &Host, hit: Hit) -> Option<(String, String)> {
    let f = host.flashing.as_ref()?;
    Some(match hit {
        Hit::FlashTick(i) => {
            let l = f.choice.lines.get(i)?;
            (l.head(), format!("{}. Click to tick it or leave it out.", l.state()))
        }
        Hit::FlashType => ("Find".into(), format!("A device's name, or conditions: {}. Enter applies it.", deck_show::choose::FILTER_HINT)),
        Hit::FlashAll => ("All".into(), "Ticks every device.".into()),
        Hit::FlashNone => ("None".into(), "Leaves every device out.".into()),
        Hit::FlashForce(on) => (if on { "Forced" } else { "A suggestion" }.into(), deck_show::choose::forced_words(on)),
        Hit::FlashLock(on) => (if on { "Locked" } else { "Not locked" }.into(), deck_show::choose::locked_words(on)),
        Hit::FlashAsk => ("Flash".into(), "On to the confirm: nothing is sent before it.".into()),
        Hit::FlashConfirm => ("Flash".into(), "Sends the show to the devices ticked.".into()),
        _ => return None,
    })
}
