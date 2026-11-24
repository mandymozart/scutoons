//! The Session sheet (#531; agents/research/SCENE-EVENTS.md §6.3, §6.7): the editor as conductor offers its session to a
//! linked reader – the code and the link's state, the place a reader starts at (a toon, a chapter, one of its labels), the link and its QR, Copy link, Open here; then the linked readers and the last story heard;
//! then the session's live cameras, who holds each, and the QR of their list a phone scans.
//! What it says is deck-ui `session::Panel`, filled by deck-space `session_panel` (the core's `reader_link`, the relay
//! client's `Readers`, the QR); this file fetches the public library's lists (HTTP), lays the panel out and does what a
//! press says: the clipboard and the system's opener are the platform's (periphery).
use super::{Hit, Host};
use deck_kit::base::draw::{self, DrawList};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::linked::Readers;
use deck_space::session_panel::{self, Facts};
use deck_ui::session::{camera_line, reader_line, Panel, Place};
use std::time::Duration;

/// What the sheet keeps: the place picked, what there is to pick, a check run's readers.
#[derive(Debug, Default)]
pub struct Session {
    pub place: Place,
    chapters: Vec<u32>,
    /// the picked chapter's jump places (the core's `places`)
    labels: Vec<String>,
    /// a check run's linked readers (`--linked`), in place of the relay's
    pub demo: Option<Readers>,
    /// a check run's live cameras (`--cameras`), in place of the relay's
    pub demo_cameras: Option<Vec<protocol::LiveCamera>>,
    /// the live camera picked in the list (its id): the QR is its own
    pub camera: Option<String>,
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
        let heard = self.link.as_ref().map(|l| l.cameras.lock().unwrap().clone());
        let cameras = self.session.demo_cameras.clone().or(heard).unwrap_or_default();
        session_panel::panel(&Facts { base: &base, code, relay, state: &state, place: &s.place, toons: &self.toons.list, chapters: &s.chapters, labels: &s.labels, readers: &readers, cameras: &cameras, picked: self.session.camera.as_deref() })
    }

    /// the sheet opened: the toons asked; the place starts at the show's toon
    pub(super) fn session_open(&mut self) {
        self.toons_ask();
        if self.session.place.toon.is_empty() {
            if let Some(t) = self.show_toon().map(str::to_string) {
                self.session_toon(&t);
            }
        }
    }

    /// a toon picked: its listed chapters; the chapter and the label start over
    pub(super) fn session_toon(&mut self, toon: &str) {
        self.session.place = Place { toon: toon.to_string(), chapter: None, label: String::new() };
        self.session.chapters.clear();
        self.session.labels.clear();
        match get(&session_panel::toon_url(&self.api.base, toon)).and_then(|t| session_panel::chapters(&t)) {
            Ok(c) => self.session.chapters = c,
            Err(e) => self.refuse(format!("{toon}: {e}")),
        }
    }

    /// the picked chapter's jump places, from the public library's chapter (the core reads them)
    pub(super) fn session_labels(&mut self, chapter: Option<u32>) {
        self.session.labels.clear();
        let (Some(n), toon) = (chapter, self.session.place.toon.clone()) else { return };
        match get(&session_panel::chapter_url(&self.api.base, &toon, n)).and_then(|t| session_panel::labels(&t)) {
            Ok(l) => self.session.labels = l,
            Err(e) => self.refuse(format!("{toon} {n}: {e}")),
        }
    }

    pub(super) fn session_press(&mut self, hit: Hit) {
        match hit {
            Hit::SessionToon(i) => {
                if let Some(t) = self.toons.list.get(i).cloned() {
                    if self.show_toon().is_none() {
                        self.edit_rows(&deck_show::rows::Edit::Toon { to: t.clone() });
                    }
                    self.session_toon(&t);
                }
            }
            Hit::SessionChapter(i) => {
                if let Some(n) = self.session.chapters.get(i).copied() {
                    self.session.place.chapter = Some(n);
                    self.session.place.label.clear();
                    self.session_labels(Some(n));
                }
            }
            // the first chip is the chapter's start, the others its labels
            Hit::SessionLabel(i) => self.session.place.label = i.checked_sub(1).and_then(|l| self.session.labels.get(l).cloned()).unwrap_or_default(),
            Hit::SessionCamera(i) => {
                let id = self.session_panel().cameras.get(i).map(|c| c.id.clone());
                self.session.camera = id.filter(|id| self.session.camera.as_ref() != Some(id));
            }
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

impl Host {
    /// a check run (`--cameras ID=HOLDER,ID,ID=HOLDER~`): the session's cameras as the relay would list them – a holder by
    /// its name, `~` away
    pub fn session_cameras_demo(&mut self, cameras: &[(String, Option<String>)]) {
        let list = cameras.iter().enumerate().map(|(i, (id, who))| {
            let away = who.as_deref().is_some_and(|w| w.ends_with('~'));
            let nick = who.as_deref().map(|w| w.trim_end_matches('~').to_string());
            serde_json::json!({ "id": id, "holder": nick.as_ref().map(|_| format!("c{}", i + 4)), "nick": nick, "away": away.then_some(true) })
        });
        self.session.demo_cameras = serde_json::from_value(serde_json::Value::Array(list.collect())).ok();
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
    // the label: the chapter's start, or one of its labels (the core's jump places)
    let names: Vec<String> = if p.place.chapter.is_some() { std::iter::once("start".to_string()).chain(p.labels.iter().cloned()).collect() } else { Vec::new() };
    let label_on = if p.place.label.is_empty() { Some(0) } else { p.labels.iter().position(|l| *l == p.place.label).map(|i| i + 1) };
    let (rects, used) = pick(d, t, (x, y, cw), "Label", &names, label_on, "pick a chapter");
    host.hits.extend(rects.into_iter().enumerate().map(|(i, r)| (r, Hit::SessionLabel(i))));
    y += used + pad * 0.5;
    d.text(x, y, font, &fit(d, &p.place_line(), font, cw), t.color.text);
    y += font * 1.6;
    d.text(x, y, small, &fit(d, &p.link_line(), small, cw), if p.has_link() { t.color.text } else { t.color.muted });
    y += small * 1.8;
    // Copy link, Open here
    let ink = if p.has_link() { t.color.text } else { t.color.muted };
    let mut bx = x;
    for (word, hit) in [("Copy link", Hit::SessionCopy), ("Open here", Hit::SessionOpen)] {
        let r = Rect { x: bx, y, w: d.advance(font * 1.3) * word.chars().count() as f32 + 2.0 * pad, h: row };
        super::live::button(d, t, r, word, t.color.control, ink);
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

    // the linked readers now and the last story heard on the left; the live cameras, who holds each, and their list's QR
    // a phone scans on the right
    d.rect(a.x, y - 4.0, a.w, 1.0, t.color.line);
    let split = a.x + a.w * 0.4;
    let col = a.w * 0.4 - 2.0 * pad;
    let below = y;
    d.text(x, y + 4.0, small, &fit(d, &p.readers_line().to_uppercase(), small, col), t.color.muted);
    y += row;
    for (i, r) in p.readers.iter().enumerate() {
        if y + row * 2.0 > a.y + a.h {
            d.text(x, y + 2.0, small, &format!("{} more …", p.readers.len() - i), t.color.muted);
            y += row;
            break;
        }
        if r.drives {
            d.disc(x + 4.0, y + row * 0.5, 4.0, t.color.go);
        }
        d.text(x + 16.0, y + (row - font * 1.25) * 0.5, font, &fit(d, &reader_line(r), font, col - 16.0), t.color.text);
        y += row;
    }
    d.text(x, y + 4.0, font, &fit(d, &p.last_line(), font, col), if p.last.is_some() { t.color.text } else { t.color.muted });
    if p.code.is_empty() {
        return;
    }
    let cx = split + pad;
    let col = a.x + a.w - cx - pad;
    let mut y = below;
    d.rect(split, below - 4.0, 1.0, (a.y + a.h - below + 4.0).max(0.0), t.color.line);
    d.text(cx, y + 4.0, small, &fit(d, &format!("LIVE CAMERAS · {}", p.cameras_line().to_uppercase()), small, col), t.color.muted);
    y += row;
    let side = (a.y + a.h - y - pad).clamp(0.0, 96.0);
    let lw = if p.cameras_qr.n > 0 && side >= 72.0 {
        let module = (side / (p.cameras_qr.n + 8) as f32).floor().max(1.0);
        draw::qr(d, (a.x + a.w - side - pad, y), module, p.cameras_qr.n, &|mx, my| p.cameras_qr.dark.get(my * p.cameras_qr.n + mx).copied().unwrap_or(false));
        col - side - pad
    } else {
        col
    };
    for (i, c) in p.cameras.iter().enumerate() {
        if y + row > a.y + a.h {
            d.text(cx, y + 2.0, small, &format!("{} more …", p.cameras.len() - i), t.color.muted);
            break;
        }
        let r = Rect { x: cx, y, w: lw, h: row };
        if p.picked == Some(i) {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.control);
        }
        if c.holder.is_some() {
            d.disc(cx + 4.0, y + row * 0.5, 4.0, if c.away { t.color.muted } else { t.color.go });
        }
        d.text(cx + 16.0, y + (row - small * 1.25) * 0.5, small, &fit(d, &camera_line(c), small, lw - 16.0), t.color.text);
        host.hits.push((r, Hit::SessionCamera(i)));
        y += row;
    }
    d.text(cx, y + 4.0, small, &fit(d, &p.cameras_link, small, lw), t.color.text);
}
