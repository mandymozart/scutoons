//! A newer version announces itself (RULES H19; the server's contract: API.md "Is there a newer version", #858): the
//! editor asks for the newest version of the open show on the window's focus and every 30 s, with the version it holds
//! as `If-None-Match` – an unchanged answer costs nothing. A newer one shows the notice above the status line with
//! Reload; Reload reads the server's newest, unsent changes still go up first, and offline the cached copy opens and the
//! status says why. Its own save moves the held version on, so it never announces itself.
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

/// how often the open show is asked for while nothing else asks
pub const EVERY: Duration = Duration::from_secs(30);

type Answer = (String, Result<Option<u64>, String>);

/// What the editor knows of the server's newest versions.
#[derive(Default)]
pub struct Newer {
    /// a show's id and the newest version the server named for it
    found: Option<(String, u64)>,
    asking: Option<Receiver<Answer>>,
    last: Option<Instant>,
    /// the window got the focus: ask at once
    pub focus: bool,
}

impl Host {
    /// the server's version of show `id` this editor holds (none: not read from the server)
    fn held(&self, id: &str) -> Option<u64> {
        self.saves.get(id).and_then(|a| a.version)
    }

    /// each frame: the answer taken in; asked again on the focus or every `EVERY`
    pub(super) fn newer_poll(&mut self) {
        if let Some(Ok((id, answer))) = self.newer.asking.as_ref().map(Receiver::try_recv) {
            self.newer.asking = None;
            match answer {
                Ok(Some(v)) => self.newer.found = Some((id, v)),
                Ok(None) => self.newer.found = self.newer.found.take().filter(|(f, _)| *f != id),
                // offline or refused: nothing to say until it answers
                Err(_) => {}
            }
        }
        let id = self.shown().id.clone();
        let Some(held) = self.held(&id) else { return };
        let due = self.newer.focus || self.newer.last.is_none_or(|l| l.elapsed() >= EVERY);
        if !due || self.newer.asking.is_some() {
            return;
        }
        (self.newer.focus, self.newer.last) = (false, Some(Instant::now()));
        let (tx, rx) = channel();
        let api = self.api.clone();
        std::thread::spawn(move || {
            let _ = tx.send((id.clone(), api.newest(&id, held)));
        });
        self.newer.asking = Some(rx);
    }

    /// the newer version of the show shown the server has, if any
    pub fn newer_version(&self) -> Option<u64> {
        let id = &self.shown().id;
        self.newer.found.as_ref().filter(|(f, v)| f == id && self.held(f) != Some(*v)).map(|(_, v)| *v)
    }

    /// A check run: the server said `version` is the newest of the show shown (no server needed)
    pub fn script_newer(&mut self, version: u64) {
        self.newer.found = Some((self.shown().id.clone(), version));
    }
}

/// what the notice says
pub fn words(id: &str) -> String {
    format!("A newer version of {id} exists")
}

/// The notice above the status line, at the left of a window `room` big: a dot, its words, Reload.
pub fn notice(d: &mut DrawList, t: &Tokens, host: &mut Host, room: (f32, f32), pointer: (f32, f32)) {
    if host.newer_version().is_none() {
        return;
    }
    let (pad, px) = (t.size.pad, t.size.font);
    let h = t.size.row;
    let inset = t.size.rem(0.22);
    let text = words(&host.shown().id);
    let bw = d.sans_width(px, "Reload") + pad * 3.0;
    let w = pad * 1.2 + t.size.rem(0.5) + pad * 0.6 + d.sans_width(px, &text) + pad + bw + inset;
    let r = Rect { x: t.size.gap, y: room.1 - deck_kit::editor::status::reserve(t) - t.size.gap * 0.5 - h, w, h };
    let s = t.kit("pill", "rest");
    d.round_rect(r.x, r.y, r.w, r.h, h * 0.5, s.ground.unwrap_or(t.color.panel()));
    d.outline(r.x, r.y, r.w, r.h, (h * 0.5, t.size.edge), s.border.unwrap_or(t.color.line));
    let halo = d.halo_state();
    d.halo(t.color.ring());
    let mut x = r.x + pad * 1.2;
    let dot = t.size.rem(0.25);
    d.disc(x + dot, r.y + h * 0.5, dot, t.color.warn);
    x += dot * 2.0 + pad * 0.6;
    d.sans(x, r.y + (h - px * 1.25) * 0.5, px, &text, t.color.text);
    let b = Rect { x: r.x + r.w - inset - bw, y: r.y + inset, w: bw, h: h - 2.0 * inset };
    controls::button(d, t, (b.x, b.y, b.h), "Reload", "button_plain", if b.has(pointer.0, pointer.1) { "hover" } else { "rest" });
    d.restore_halo(halo);
    host.hits.push((r, Hit::Ground));
    host.hits.push((b, Hit::Reload));
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::host_with;
    use super::super::Hit;

    const DOC: &str = r#"{ "id": "top", "version": 3, "toons": ["catpope"], "outlets": [{ "id": "left" }], "rows": [] }"#;

    /// H19: a version the server names that this editor does not hold shows the notice with Reload; the held one does not
    #[test]
    fn a_newer_version_shows_the_notice_and_the_held_one_does_not() {
        let mut host = host_with(DOC);
        host.saves.insert("top".into(), crate::autosave::Autosave::new(Some(41)));
        assert_eq!(host.newer_version(), None);
        host.script_newer(41);
        assert_eq!(host.newer_version(), None, "its own version is no news");
        host.script_newer(42);
        assert_eq!(host.newer_version(), Some(42));
        let (mut d, t) = (deck_kit::base::draw::DrawList::new(0.6), deck_kit::base::tokens::Tokens::default());
        host.hits.clear();
        super::notice(&mut d, &t, &mut host, (1600.0, 900.0), (0.0, 0.0));
        assert!(d.texts().contains(&"A newer version of top exists"), "{:?}", d.texts());
        assert!(host.hits.iter().any(|(_, h)| *h == Hit::Reload));
        // a save of its own, or a reload, moves the held version on: the notice goes
        host.saves.insert("top".into(), crate::autosave::Autosave::new(Some(42)));
        assert_eq!(host.newer_version(), None);
    }
}
