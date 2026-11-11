//! What a deck says to the person at it about a flash (#481, draft 1): a dialog – a suggestion to accept or reject, the
//! notice after a flash replaced the deck's own version, "please don't flash" with its reason – or a toast that goes by
//! itself. Plain data and the words, German or English by the deck's locale; `face.rs` draws it with the component
//! library (deck-kit `panel.rs`) and hands what was pressed back (`Act`, `Key`). The rules are `flashed.rs`'s.
use crate::document::Pinned;
use crate::ready::Kept;
pub use deck_kit::panel::Tone;
use deck_kit::text::Field;

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
    /// "please don't flash" as it was set
    Save,
    /// leave it as it was
    Cancel,
}

/// A key of a deck without touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    /// to the row or button before, after (Tab is Down)
    Up,
    Down,
    Enter,
    Escape,
    Backspace,
    Delete,
    Home,
    End,
}

/// A row of a dialog, above its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    /// its label, on or off
    Switch(String, bool),
    /// one line of text; `cursor`: the characters before the cursor while it is edited
    Field { text: String, hint: String, cursor: Option<usize> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    /// what this is, and who it is from
    pub caption: (String, String),
    /// none = no title
    pub title: String,
    pub rows: Vec<Row>,
    pub lines: Vec<String>,
    /// side by side, the first one the primary
    pub buttons: Vec<(String, Act)>,
    /// what the keys are on: the rows top down, then the buttons
    pub focus: usize,
    /// 0..1 of the time gone after which the primary happens by itself
    pub gone: Option<f32>,
    /// what the keys do: (key, what)
    pub keys: Vec<(&'static str, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    Dialog(Dialog),
    /// no question: what happened (owner 2026-11-11: a toast, like the reader's)
    Toast(String, Tone),
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
        rows: vec![],
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
        rows: vec![],
        lines,
        focus: focus.min(buttons.len() - 1),
        keys: keys(de, "OK", None, buttons.len() > 1),
        buttons,
        gone: None,
    }
}

/// "Please don't flash" and why, to set at the deck (draft 1, frame 5): a switch, the reason, save or cancel. `focus`:
/// 0 the switch, 1 the reason (being edited), 2 and 3 the buttons.
pub fn keep(locale: &str, runs: Option<&Pinned>, on: bool, why: &Field, focus: usize) -> Dialog {
    let de = german(locale);
    let focus = focus.min(3);
    let (text, cursor) = why.shown();
    let (save, cancel) = if de { ("Speichern", "Abbrechen") } else { ("Save", "Cancel") };
    Dialog {
        caption: (if de { "Flash · dieses Deck" } else { "Flash · this deck" }.into(), runs.map(|r| format!("{}: {}", if de { "läuft" } else { "runs" }, short(r))).unwrap_or_default()),
        title: String::new(),
        rows: vec![
            Row::Switch(if de { "Bitte nicht flashen" } else { "Please don't flash" }.into(), on),
            Row::Field { text, hint: if de { "warum" } else { "why" }.into(), cursor: (focus == 1).then_some(cursor) },
        ],
        lines: vec![if de { "Der Conductor sieht den Grund. Ein erzwungener Flash geht trotzdem durch." } else { "The conductor sees the reason. A forced flash still goes through." }.into()],
        buttons: vec![(save.into(), Act::Save), (cancel.into(), Act::Cancel)],
        focus,
        gone: None,
        keys: vec![("Enter", save.to_lowercase()), ("Esc", cancel.to_lowercase()), ("Tab", if de { "weiter" } else { "next" }.to_string())],
    }
}

/// the toast after a flash was taken (draft 1, frame 4)
pub fn flashed(locale: &str, show: &Pinned, by: Option<&str>) -> Shown {
    let de = german(locale);
    Shown::Toast(format!("{} {}: {}", if de { "Geflasht" } else { "Flashed" }, from(de, by), long(show)), Tone::Good)
}

/// the toast after a suggestion was rejected
pub fn rejected(locale: &str, show: &Pinned) -> Shown {
    Shown::Toast(format!("{}: {}", if german(locale) { "Vorschlag abgelehnt" } else { "Suggestion rejected" }, long(show)), Tone::Warn)
}

/// the toast after the way back
pub fn own_again(locale: &str, own: &Pinned) -> Shown {
    Shown::Toast(format!("{}: {}", if german(locale) { "Deine Version läuft wieder" } else { "Your version runs again" }, long(own)), Tone::Good)
}

/// the toast after the switch back to what was flashed
pub fn flashed_again(locale: &str, show: &Pinned) -> Shown {
    Shown::Toast(format!("{}: {}", if german(locale) { "Die geflashte Version läuft wieder" } else { "The flashed version runs again" }, long(show)), Tone::Good)
}

/// the toast after "please don't flash" was saved: what the conductor reads now
pub fn keep_saved(locale: &str, keep: Option<&str>) -> Shown {
    let text = match (keep, german(locale)) {
        (Some(why), true) => format!("Gespeichert: bitte nicht flashen ({why})"),
        (Some(why), false) => format!("Saved: please don't flash ({why})"),
        (None, true) => "Gespeichert: dieses Deck darf geflasht werden".into(),
        (None, false) => "Saved: this deck may be flashed".into(),
    };
    Shown::Toast(text, Tone::Good)
}

/// the toast when something did not work, in the core's words
pub fn failed(why: String) -> Shown {
    Shown::Toast(why, Tone::Bad)
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
        assert_eq!(flashed("en", &pin(3), Some("ana")), Shown::Toast("Flashed by ana: top · Version 3".into(), Tone::Good));
        assert_eq!(flashed("de", &pin(3), None), Shown::Toast("Geflasht vom Conductor: top · Version 3".into(), Tone::Good));
    }

    #[test]
    fn please_dont_flash_is_a_switch_a_reason_and_two_buttons() {
        let why = Field::with("Probe läuft");
        let d = keep("de", Some(&pin(2)), true, &why, 1);
        assert_eq!(d.caption, ("Flash · dieses Deck".to_string(), "läuft: top · 2".to_string()));
        assert_eq!(d.rows, [Row::Switch("Bitte nicht flashen".into(), true), Row::Field { text: "Probe läuft".into(), hint: "warum".into(), cursor: Some(11) }]);
        assert_eq!((d.title.as_str(), d.buttons.len(), d.focus), ("", 2, 1));
        // the reason shows its cursor only while the keys are on it
        let off = keep("en", None, false, &why, 0);
        assert_eq!(off.rows[1], Row::Field { text: "Probe läuft".into(), hint: "why".into(), cursor: None });
        assert_eq!(keep_saved("de", Some("Probe läuft")), Shown::Toast("Gespeichert: bitte nicht flashen (Probe läuft)".into(), Tone::Good));
    }
}
