//! The deck editor's Session panel (#531; agents/research/SCENE-EVENTS.md §6.3, §6.7): what a conductor shows of its live
//! session – the code and the link's state, the row **Linked reader** (the place a reader starts at: a toon, a chapter of
//! it, a label in it; the link the core makes of it; its QR), then the linked readers now (which one drives) and the last
//! story event heard. Plain data in, the panel's words out, no window: deck-space `session_panel.rs` fills a [`Panel`]
//! from the relay client's `Readers` and the core's `reader_link`; the editor draws it and does what a press says.
use crate::menu::Qr;

/// Where a linked reader starts: a toon (its id), a chapter of it (its number), a label in it (empty: the chapter's start).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Place {
    pub toon: String,
    pub chapter: Option<u32>,
    pub label: String,
}

/// A linked reader now: its client id, the name it said (none: none), whether it drives the session's story.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reader {
    pub id: String,
    pub name: Option<String>,
    pub drives: bool,
}

/// The last story event the conductor heard: its type, the characters it names, the reader (client id) that sent it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Heard {
    pub kind: String,
    pub characters: Vec<String>,
    pub from: String,
}

/// The panel: the session, the place picked with what there is to pick, the link and its QR, the readers, the last event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Panel {
    /// the session's code (`A7K2`)
    pub code: String,
    /// the relay client's line (joining, connected as, offline …)
    pub state: String,
    pub place: Place,
    /// the toons to pick (the public library's ids) and the picked toon's chapters (the numbers the library lists)
    pub toons: Vec<String>,
    pub chapters: Vec<u32>,
    /// the address a linked reader opens – empty until a toon and a chapter are picked
    pub link: String,
    /// the link's QR (empty with no link)
    pub qr: Qr,
    pub readers: Vec<Reader>,
    pub last: Option<Heard>,
}

/// the words before a place is whole
pub const PICK: &str = "pick a toon and a chapter";
/// the words without a session (the editor joins one as its conductor with `--relay`)
pub const NO_SESSION: &str = "no session – join one with --relay ws://host:port/live/CODE";

/// a reader as people read it: its name, else its client id
pub fn who(r: &Reader) -> String {
    r.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).map_or_else(|| r.id.clone(), str::to_string)
}

/// a reader's line in the list: `Mira (c4) · drives`
pub fn reader_line(r: &Reader) -> String {
    let name = match r.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => format!("{n} ({})", r.id),
        None => r.id.clone(),
    };
    if r.drives {
        format!("{name} · drives")
    } else {
        name
    }
}

impl Panel {
    /// the reader that drives the story now, by name (none: nobody drives)
    pub fn driver(&self) -> Option<String> {
        self.readers.iter().find(|r| r.drives).map(who)
    }

    /// the list's head: `2 linked readers · Mira drives`, `1 linked reader · nobody drives`, `no linked reader`
    pub fn readers_line(&self) -> String {
        match self.readers.len() {
            0 => "no linked reader".into(),
            n => {
                let many = if n == 1 { "1 linked reader".to_string() } else { format!("{n} linked readers") };
                match self.driver() {
                    Some(d) => format!("{many} · {d} drives"),
                    None => format!("{many} · nobody drives"),
                }
            }
        }
    }

    /// the last story event: `last: joinScene · character:catpope · from Mira` (the reader by its name while it is in)
    pub fn last_line(&self) -> String {
        let Some(h) = &self.last else { return "no story heard yet".into() };
        let from = self.readers.iter().find(|r| r.id == h.from).map_or_else(|| h.from.clone(), who);
        let mut parts = vec![if h.kind.is_empty() { "an event".to_string() } else { h.kind.clone() }];
        if !h.characters.is_empty() {
            parts.push(h.characters.join(", "));
        }
        parts.push(format!("from {from}"));
        format!("last: {}", parts.join(" · "))
    }

    /// the place picked: `catpope · chapter 3 · from dialog-start` (`· from the start` without a label), else [`PICK`]
    pub fn place_line(&self) -> String {
        let p = &self.place;
        match (p.toon.trim(), p.chapter) {
            ("", _) | (_, None) => PICK.into(),
            (toon, Some(n)) => match p.label.trim() {
                "" => format!("{toon} · chapter {n} · from the start"),
                l => format!("{toon} · chapter {n} · from {l}"),
            },
        }
    }

    /// what the link row says: the link, or what is missing (a session, a place)
    pub fn link_line(&self) -> String {
        match (self.link.is_empty(), self.code.trim().is_empty()) {
            (false, _) => self.link.clone(),
            (true, true) => NO_SESSION.into(),
            (true, false) => PICK.into(),
        }
    }

    /// Copy link and Open here want a link
    pub fn has_link(&self) -> bool {
        !self.link.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Panel {
        Panel {
            code: "A7K2".into(),
            readers: vec![Reader { id: "c4".into(), name: Some("Mira".into()), drives: true }, Reader { id: "c5".into(), name: None, drives: false }],
            last: Some(Heard { kind: "joinScene".into(), characters: vec!["character:catpope".into()], from: "c4".into() }),
            ..Panel::default()
        }
    }

    #[test]
    fn the_panel_names_the_driver_and_the_last_event() {
        let p = panel();
        assert_eq!(p.driver().as_deref(), Some("Mira"));
        assert_eq!(p.readers_line(), "2 linked readers · Mira drives");
        assert_eq!(p.readers.iter().map(reader_line).collect::<Vec<_>>(), ["Mira (c4) · drives", "c5"]);
        assert_eq!(p.last_line(), "last: joinScene · character:catpope · from Mira");
        let mut gone = p.clone();
        gone.readers.clear();
        assert_eq!((gone.readers_line(), gone.driver()), ("no linked reader".to_string(), None));
        assert_eq!(gone.last_line(), "last: joinScene · character:catpope · from c4", "a reader that left: its id");
        assert_eq!(Panel::default().last_line(), "no story heard yet");
        let mut none = p;
        none.readers.truncate(1);
        none.readers[0].drives = false;
        assert_eq!(none.readers_line(), "1 linked reader · nobody drives");
    }

    #[test]
    fn the_place_and_the_link_say_what_is_missing() {
        let mut p = Panel::default();
        assert_eq!((p.place_line().as_str(), p.link_line().as_str(), p.has_link()), (PICK, NO_SESSION, false));
        p.code = "A7K2".into();
        assert_eq!(p.link_line(), PICK);
        p.place = Place { toon: "catpope".into(), chapter: Some(3), label: String::new() };
        assert_eq!(p.place_line(), "catpope · chapter 3 · from the start");
        p.place.label = "dialog-start".into();
        assert_eq!(p.place_line(), "catpope · chapter 3 · from dialog-start");
        p.link = "https://scutoons.com/catpope/chapter/3?label=dialog-start&live=A7K2".into();
        assert!(p.has_link());
        assert_eq!(p.link_line(), p.link);
    }
}
