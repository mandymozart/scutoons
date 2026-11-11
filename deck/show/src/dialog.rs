//! What a deck says to the person at it about a flash (#481, draft 1): a dialog – a suggestion to accept or reject, or
//! the notice after a flash replaced the deck's own version – or one line that goes by itself. Plain data and the words,
//! German or English by the deck's locale; a platform draws it with the component library (deck-kit `panel.rs`) and
//! hands what was pressed back (`Act`, `Key`). The rules are `flashed.rs`'s.
use crate::document::Pinned;
use crate::ready::Kept;

/// What a button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// take the suggested flash
    Accept,
    /// keep what runs
    Reject,
    /// the notice was read
    Seen,
    /// back to the deck's own version
    Back,
}

/// A key of a deck without touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Enter,
    Escape,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    /// what this is, and who it is from
    pub caption: (String, String),
    pub title: String,
    pub lines: Vec<String>,
    /// side by side, the first one the primary
    pub buttons: Vec<(String, Act)>,
    /// the button the keys are on
    pub focus: usize,
    /// 0..1 of the time gone after which the primary happens by itself
    pub gone: Option<f32>,
    /// what the keys do: (key, what)
    pub keys: Vec<(&'static str, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    Dialog(Dialog),
    /// no question: what happened, and who did it
    Line(String, String),
}

fn german(locale: &str) -> bool {
    locale.starts_with("de")
}

/// `top · Version 3`
fn long(p: &Pinned) -> String {
    format!("{} · Version {}", p.id, p.version)
}

/// `top · 2`
fn short(p: &Pinned) -> String {
    format!("{} · {}", p.id, p.version)
}

/// who flashed; no name (or the core's stand-in for none) = the conductor
fn from(de: bool, by: Option<&str>) -> String {
    match (by.filter(|b| !b.is_empty() && *b != "the conductor"), de) {
        (Some(b), true) => format!("von {b}"),
        (Some(b), false) => format!("by {b}"),
        (None, true) => "vom Conductor".into(),
        (None, false) => "by the conductor".into(),
    }
}

fn keys(de: bool, primary: &str, escape: Option<&str>, two: bool) -> Vec<(&'static str, String)> {
    let mut out = vec![("Enter", primary.to_string())];
    out.extend(escape.map(|e| ("Esc", e.to_string())));
    if two {
        out.push(("< >", if de { "wechseln" } else { "switch" }.to_string()));
    }
    out
}

/// A flash that is only suggested: accept or reject; `gone` of its time is over, then it is accepted (draft 1, frame 1).
pub fn suggestion(locale: &str, show: &Pinned, by: Option<&str>, runs: Option<&Pinned>, gone: f32, focus: usize) -> Dialog {
    let de = german(locale);
    let line = match (runs.filter(|r| !r.id.is_empty()), de) {
        (Some(r), true) => format!("Du spielst deine eigene Version ({}). Sie bleibt behalten.", short(r)),
        (Some(r), false) => format!("You play your own version ({}). It is kept.", short(r)),
        (None, true) => "Auf diesem Deck läuft noch keine Show.".into(),
        (None, false) => "No show runs on this deck yet.".into(),
    };
    let (accept, reject) = if de { ("Annehmen", "Ablehnen") } else { ("Accept", "Reject") };
    Dialog {
        caption: (if de { "Flash · Vorschlag" } else { "Flash · suggestion" }.into(), from(de, by)),
        title: long(show),
        lines: vec![line],
        buttons: vec![(accept.into(), Act::Accept), (reject.into(), Act::Reject)],
        focus: focus.min(1),
        gone: Some(gone.clamp(0.0, 1.0)),
        keys: keys(de, &accept.to_lowercase(), Some(&reject.to_lowercase()), true),
    }
}

/// A flash replaced the deck's own version: what runs, that the own one is kept, and the way back when there is one
/// (draft 1, frame 3).
pub fn notice(locale: &str, kept: &Kept, forced: bool, may_switch: bool, focus: usize) -> Dialog {
    let de = german(locale);
    let caption = match (forced || kept.forced, de) {
        (true, true) => "Flash · erzwungen",
        (true, false) => "Flash · forced",
        (false, _) => "Flash",
    };
    let mut lines = vec![if de { format!("Deine Version ({}) ist behalten.", short(&kept.own)) } else { format!("Your version ({}) is kept.", short(&kept.own)) }];
    let mut buttons = vec![("OK".to_string(), Act::Seen)];
    if may_switch {
        buttons.push((if de { "Zurück zu meiner Version" } else { "Back to my version" }.into(), Act::Back));
    } else {
        lines.push(if de { "Gesperrt: nur der Besitzer des Decks kann zurück." } else { "Locked: only the deck's owner can go back." }.into());
    }
    Dialog {
        caption: (caption.into(), from(de, Some(&kept.by))),
        title: if de { format!("{} läuft", long(&kept.flashed)) } else { format!("{} runs", long(&kept.flashed)) },
        lines,
        focus: focus.min(buttons.len() - 1),
        keys: keys(de, "OK", None, buttons.len() > 1),
        buttons,
        gone: None,
    }
}

/// the one line after a flash was taken (draft 1, frame 4)
pub fn flashed(locale: &str, show: &Pinned, by: Option<&str>) -> Shown {
    let de = german(locale);
    Shown::Line(format!("{}: {}", if de { "Geflasht" } else { "Flashed" }, long(show)), from(de, by))
}

/// the one line after a suggestion was rejected
pub fn rejected(locale: &str, show: &Pinned) -> Shown {
    Shown::Line(format!("{}: {}", if german(locale) { "Vorschlag abgelehnt" } else { "Suggestion rejected" }, long(show)), String::new())
}

/// the one line after the way back
pub fn own_again(locale: &str, own: &Pinned) -> Shown {
    Shown::Line(format!("{}: {}", if german(locale) { "Deine Version läuft wieder" } else { "Your version runs again" }, long(own)), String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(v: u32) -> Pinned {
        Pinned { id: "top".into(), version: v }
    }

    #[test]
    fn a_suggestion_says_what_comes_what_stays_and_counts_down() {
        let d = suggestion("de", &pin(3), Some("ana"), Some(&pin(2)), 0.4, 0);
        assert_eq!(d.caption, ("Flash · Vorschlag".to_string(), "von ana".to_string()));
        assert_eq!((d.title.as_str(), d.lines[0].as_str()), ("top · Version 3", "Du spielst deine eigene Version (top · 2). Sie bleibt behalten."));
        assert_eq!(d.buttons, [("Annehmen".to_string(), Act::Accept), ("Ablehnen".to_string(), Act::Reject)]);
        assert_eq!((d.gone, d.keys[0].1.as_str(), d.keys.len()), (Some(0.4), "annehmen", 3));
        // English, nobody named, nothing running yet
        let e = suggestion("en", &pin(3), None, None, 2.0, 5);
        assert_eq!((e.caption.1.as_str(), e.lines[0].as_str(), e.focus, e.gone), ("by the conductor", "No show runs on this deck yet.", 1, Some(1.0)));
    }

    #[test]
    fn a_notice_offers_the_way_back_only_when_there_is_one() {
        let kept = Kept { own: pin(2), flashed: pin(3), by: "ana".into(), forced: true, locked: false, on_own: false };
        let d = notice("de", &kept, false, true, 0);
        assert_eq!((d.caption.0.as_str(), d.title.as_str()), ("Flash · erzwungen", "top · Version 3 läuft"));
        assert_eq!(d.buttons, [("OK".to_string(), Act::Seen), ("Zurück zu meiner Version".to_string(), Act::Back)]);
        let locked = notice("de", &Kept { forced: false, by: "the conductor".into(), ..kept }, false, false, 3);
        assert_eq!((locked.caption, locked.buttons.len(), locked.focus), (("Flash".to_string(), "vom Conductor".to_string()), 1, 0));
        assert_eq!(locked.lines[1], "Gesperrt: nur der Besitzer des Decks kann zurück.");
        assert_eq!(flashed("en", &pin(3), Some("ana")), Shown::Line("Flashed: top · Version 3".into(), "by ana".into()));
    }
}
