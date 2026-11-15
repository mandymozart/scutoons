//! The Session sheet (#531; agents/research/SCENE-EVENTS.md §6.3, §6.7): the editor as conductor offers its session to a
//! linked reader – the code and the link's state, the place a reader starts at (a toon, a chapter, a label typed; the
//! label picker is a ticket), the link and its QR, Copy link, Open here; then the linked readers and the last story heard.
//! What it says is deck-ui `session::Panel`, filled by deck-space `session_panel` (the core's `reader_link`, the relay
//! client's `Readers`, the QR); this file fetches the public library's lists (HTTP), lays the panel out and does what a
//! press says: the clipboard and the system's opener are the platform's (periphery).
use super::roles::Typing;
use super::{Hit, Host};
use deck_kit::base::draw::{self, DrawList};
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::linked::Readers;
use deck_space::session_panel::{self, Facts};
use deck_ui::session::{reader_line, Panel, Place};
use std::time::Duration;

/// What the sheet keeps: the place picked, what there is to pick, a check run's readers.
#[derive(Debug, Default)]
pub struct Session {
    pub place: Place,
    toons: Vec<String>,
    chapters: Vec<u32>,
    /// the toons were asked for (once; Load asks again)
    asked: bool,
    /// a check run's linked readers (`--linked`), in place of the relay's
    pub demo: Option<Readers>,
}

/// the public library's answer at `url` (the reader's own lists, no token)
fn get(url: &str) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build();
    agent.get(url).call().map_err(|e| format!("{url}: {e}"))?.into_string().map_err(|e| format!("{url}: {e}"))
}

/// The system's way to open an address on this machine (the conductor reads and drives itself).
fn open(url: &str) -> std::io::Result<std::process::Child> {
    #[cfg(target_os = "windows")]
    let mut c = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(target_os = "macos")]
    let mut c = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut c = std::process::Command::new("xdg-open");
    c.arg(url).spawn()
}

impl Host {
    /// the panel as the core words it, for what the sheet knows now
    pub fn session_panel(&self) -> Panel {
        let live = self.link.as_ref().map(|l| l.readers.lock().unwrap().clone());
        let readers = self.session.demo.clone().or(live).unwrap_or_default();
        let code = self.link.as_ref().map_or("", |l| l.code.as_str());
        let relay = self.link.as_ref().map_or("", |l| l.relay.as_str());
        let base = deck_link::pairing::app_url();
        let s = &self.session;
        let state = self.link_line();
        session_panel::panel(&Facts { base: &base, code, relay, state: &state, place: &s.place, toons: &s.toons, chapters: &s.chapters, readers: &readers })
    }

    /// the toon the Session sheet picked (the editor's book follows it in Show and Perform)
    pub fn place_toon(&self) -> Option<&str> {
        Some(self.session.place.toon.as_str()).filter(|t| !t.is_empty())
    }

    /// the sheet opened: the toons, once
    pub(super) fn session_open(&mut self) {
        if !self.session.asked {
            self.session.asked = true;
            match get(&session_panel::toons_url(&self.api.base)).and_then(|t| session_panel::toons(&t)) {
                Ok(t) => self.session.toons = t,
                Err(e) => self.status = format!("the toons: {e}"),
            }
        }
    }

    /// a toon picked: its listed chapters; the chapter and the label start over
    pub(super) fn session_toon(&mut self, toon: &str) {
        self.session.place = Place { toon: toon.to_string(), chapter: None, label: String::new() };
        self.session.chapters.clear();
        match get(&session_panel::toon_url(&self.api.base, toon)).and_then(|t| session_panel::chapters(&t)) {
            Ok(c) => self.session.chapters = c,
            Err(e) => self.status = format!("{toon}: {e}"),
        }
    }

    pub(super) fn session_press(&mut self, hit: Hit) {
        match hit {
            Hit::SessionToon(i) => {
                if let Some(t) = self.session.toons.get(i).cloned() {
                    self.session_toon(&t);
                }
            }
            Hit::SessionChapter(i) => {
                if let Some(n) = self.session.chapters.get(i).copied() {
                    self.session.place.chapter = Some(n);
                }
            }
            Hit::SessionLabel => self.typing = Some((Typing::ReaderLabel, Field::with(&self.session.place.label))),
            Hit::SessionCopy => {
                let link = self.session_panel().link;
                if link.is_empty() {
                    return;
                }
                self.status = match arboard::Clipboard::new().and_then(|mut c| c.set_text(link.clone())) {
                    Ok(()) => format!("copied {link}"),
                    Err(e) => format!("not copied: {e}"),
                };
            }
            Hit::SessionOpen => {
                let link = self.session_panel().link;
                if link.is_empty() {
                    return;
                }
                self.status = match open(&link) {
                    Ok(_) => format!("opened {link}"),
                    Err(e) => format!("not opened: {e}"),
                };
            }
            _ => {}
        }
    }

    /// a check run (`--linked NAME,…`): the readers as the relay would name them – the first drives and joined the scene
    /// with `character` – fed through the core's `Readers`
    pub fn session_demo(&mut self, names: &[String], character: &str) {
        let readers: serde_json::Map<String, serde_json::Value> = names.iter().enumerate().map(|(i, n)| (format!("c{}", i + 4), serde_json::json!({ "nick": n }))).collect();
        let presence = serde_json::json!({ "groups": {}, "ready": [], "readers": readers, "driver": (!names.is_empty()).then_some("c4") });
        let mut r = Readers::default();
        if let Ok(p) = serde_json::from_value::<protocol::Presence>(presence) {
            r.presence(&p);
        }
        if !names.is_empty() {
            let join = serde_json::json!({ "type": "joinScene", "params": { "characters": [{ "character": character }] } });
            r.heard("c4", join.as_object().unwrap_or(&serde_json::Map::new()));
        }
        self.session.demo = Some(r);
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// a picker's row: its label, then chips (one lit); their places and the height it took
fn pick(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), label: &str, names: &[String], on: Option<usize>, empty: &str) -> (Vec<Rect>, f32) {
    let (small, row, lw) = (t.size.small, t.size.tree_row, d.advance(t.size.small) * 9.0);
    d.text(x, y + (row - small * 1.25) * 0.5, small, label, t.color.muted);
    if names.is_empty() {
        d.text(x + lw, y + (row - small * 1.25) * 0.5, small, &fit(d, empty, small, w - lw), t.color.muted);
        return (Vec::new(), row);
    }
    let words: Vec<&str> = names.iter().map(String::as_str).collect();
    let lit: Vec<bool> = (0..names.len()).map(|i| Some(i) == on).collect();
    let (r, used) = controls::chips(d, t, (x + lw, y + 2.0, w - lw), &words, &lit);
    (r, used.max(row - 2.0) + 4.0)
}

/// "Session" in the stage's place.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, _pointer: (f32, f32)) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
    let p = host.session_panel();
    let mut y = a.y + pad;
    let head = if p.code.is_empty() { "SESSION".to_string() } else { format!("SESSION {} · {}", p.code, p.state) };
    d.text(a.x + pad, y, small, &fit(d, &head, small, a.w - 2.0 * pad), t.color.muted);
    y += small * 1.8;
    d.rect(a.x, y - 4.0, a.w, 1.0, t.color.line);

    // the QR on the right, the row's controls beside it
    let qr_side = (a.h * 0.5).clamp(120.0, 260.0).min(a.w * 0.35);
    let qx = a.x + a.w - qr_side - pad;
    let cw = (qx - a.x - 3.0 * pad).max(120.0);
    let x = a.x + pad;
    d.text(x, y + 4.0, small, "LINKED READER", t.color.muted);
    y += row;
    let top = y;
    let toon_on = p.toons.iter().position(|id| *id == p.place.toon);
    let (rects, used) = pick(d, t, (x, y, cw), "Toon", &p.toons, toon_on, "no toons from the library");
    host.hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, Hit::SessionToon(i))));
    y += used;
    let chapters: Vec<String> = p.chapters.iter().map(u32::to_string).collect();
    let chapter_on = p.place.chapter.and_then(|n| p.chapters.iter().position(|c| *c == n));
    let (rects, used) = pick(d, t, (x, y, cw), "Chapter", &chapters, chapter_on, if p.place.toon.is_empty() { "pick a toon" } else { "no chapters listed" });
    host.hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, Hit::SessionChapter(i))));
    y += used;
    // the label: typed (empty: the chapter's start)
    let lw = d.advance(small) * 9.0;
    d.text(x, y + (row - small * 1.25) * 0.5, small, "Label", t.color.muted);
    let field = Rect { x: x + lw, y: y + 2.0, w: (cw - lw).min(320.0), h: row - 4.0 };
    let typing = host.typing.as_ref().filter(|(what, _)| *what == Typing::ReaderLabel).map(|(_, f)| format!("{}|", f.text));
    d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
    if typing.is_some() {
        d.outline(field.x, field.y, field.w, field.h, (t.size.radius, 1.0), t.color.accent);
    }
    let (text, ink) = match (&typing, p.place.label.is_empty()) {
        (Some(s), _) => (s.clone(), t.color.text),
        (None, true) => ("the chapter's start".to_string(), t.color.muted),
        (None, false) => (p.place.label.clone(), t.color.text),
    };
    d.text(field.x + 6.0, field.y + (field.h - font * 1.25) * 0.5, font, &fit(d, &text, font, field.w - 12.0), ink);
    host.hits.push((field, Hit::SessionLabel));
    y += row + pad * 0.5;
    d.text(x, y, font, &fit(d, &p.place_line(), font, cw), t.color.text);
    y += font * 1.6;
    d.text(x, y, small, &fit(d, &p.link_line(), small, cw), if p.has_link() { t.color.accent } else { t.color.muted });
    y += small * 1.8;
    // Copy link, Open here
    let ink = if p.has_link() { t.color.text } else { t.color.muted };
    let mut bx = x;
    for (word, hit) in [("Copy link", Hit::SessionCopy), ("Open here", Hit::SessionOpen)] {
        let r = Rect { x: bx, y, w: d.advance(font * 1.3) * word.chars().count() as f32 + 2.0 * pad, h: row };
        super::perform::button(d, t, r, word, t.color.control, ink);
        if p.has_link() {
            host.hits.push((r, hit));
        }
        bx += r.w + pad;
    }
    y += row + pad;
    if p.qr.n > 0 {
        let module = (qr_side / (p.qr.n + 8) as f32).floor().max(1.0);
        draw::qr(d, (qx, top), module, p.qr.n, &|mx, my| p.qr.dark.get(my * p.qr.n + mx).copied().unwrap_or(false));
    } else {
        d.outline(qx, top, qr_side, qr_side, (t.size.radius, 1.0), t.color.line);
        d.text(qx + pad, top + qr_side * 0.5 - small, small, &fit(d, "QR", small, qr_side - 2.0 * pad), t.color.muted);
    }
    y = y.max(top + qr_side + pad);

    // the linked readers now and the last story heard
    d.rect(a.x, y - 4.0, a.w, 1.0, t.color.line);
    d.text(x, y + 4.0, small, &fit(d, &p.readers_line().to_uppercase(), small, a.w - 2.0 * pad), t.color.muted);
    y += row;
    for (i, r) in p.readers.iter().enumerate() {
        if y + row * 2.0 > a.y + a.h {
            d.text(x, y + 2.0, small, &format!("{} more …", p.readers.len() - i), t.color.muted);
            y += row;
            break;
        }
        if r.drives {
            d.disc(x + 4.0, y + row * 0.5, 4.0, t.color.accent);
        }
        d.text(x + 16.0, y + (row - font * 1.25) * 0.5, font, &fit(d, &reader_line(r), font, a.w - 2.0 * pad - 16.0), t.color.text);
        y += row;
    }
    d.text(x, y + 4.0, font, &fit(d, &p.last_line(), font, a.w - 2.0 * pad), if p.last.is_some() { t.color.text } else { t.color.muted });
}
