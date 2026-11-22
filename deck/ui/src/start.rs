//! The deck's start: an independent deck shows its start menu over the idle scene; a
//! conductor's scene or its clock drives the deck and the menu gives way (a local MIDI clock does not); a restarted deck
//! in a session rejoins it and takes its content from the relay's snapshot; a rejoining deck whose conductor is gone
//! waits ("waiting for <show>") with Leave in reach. What Play lists: the sandbox toon's demos first (its scenes and its
//! shows), then the published scenes; a show picked there plays on this deck as its own conductor ([`Showing`]). Plain values in, the stand out – the adapter reads what it hears into [`Heard`] and the menu follows
//! ([`crate::menu::Menu::follow`]).

/// the toon whose scenes are the demos (agents/SANDBOX.md)
pub const DEMO_TOON: &str = "sandbox";

/// Where the deck's clock comes from now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clock {
    /// its own (internal, Link, OSC) or none heard
    #[default]
    Own,
    /// the relay's show clock, led by another (the conductor's)
    Conductor,
    /// a MIDI clock into this deck (the venue's table-top clock)
    Midi,
}

/// What the deck hears from a conductor now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Heard {
    /// a scene the conductor pushed to this deck (`scene.sync`), or a cell for its outlet
    pub scene: bool,
    pub clock: Clock,
}

impl Heard {
    /// The conductor's scene or its clock drives the deck; a local MIDI clock does not.
    pub fn driven(&self) -> bool {
        self.scene || self.clock == Clock::Conductor
    }
}

/// How the deck came up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Begin {
    /// nothing named, no session: the idle scene and the start menu
    Independent,
    /// a scene named (deck.toml's `scene`, `--scene`): it plays, the menu stays behind Esc
    Named,
    /// a session it was in (its code, the show's name for people): it rejoins
    Rejoin(String),
}

impl Begin {
    /// `session`: the session the deck is told to be in (deck.toml's relay + outlet); `named`: a start scene is named
    pub fn of(session: Option<&str>, named: bool) -> Begin {
        match session.filter(|s| !s.trim().is_empty()) {
            Some(s) => Begin::Rejoin(s.to_string()),
            None if named => Begin::Named,
            None => Begin::Independent,
        }
    }
}

/// Where the start stands now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stand {
    /// independent: the start menu over the idle scene
    Menu,
    /// a conductor drives it: no start menu
    Driven,
    /// in its session (rejoining, the conductor there or not known yet), nothing heard yet: no start menu
    Joined,
    /// a scene named at the start plays: no start menu
    Playing,
    /// rejoining, the conductor is gone: "waiting for <show>", Leave
    Waiting(String),
}

/// The stand from how the deck came up, what it hears, whether a conductor is in its session (none: not known) and
/// whether the person at the deck left the session (Leave).
pub fn stand(begin: &Begin, heard: &Heard, conductor: Option<bool>, left: bool) -> Stand {
    if heard.driven() {
        return Stand::Driven;
    }
    match begin {
        Begin::Rejoin(show) if !left && conductor == Some(false) => Stand::Waiting(show.clone()),
        Begin::Rejoin(_) if !left => Stand::Joined,
        Begin::Named if !left => Stand::Playing,
        _ => Stand::Menu,
    }
}

/// the waiting page's line
pub fn waiting_line(show: &str) -> String {
    format!("waiting for {show}")
}

/// One published scene or show as a listing names it: its id (`scene:<id>`, `show:<id>`), its name, its toons; a kept
/// copy's version (it plays without asking; none: the published one).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Listed {
    pub id: String,
    pub name: String,
    pub toons: Vec<String>,
    pub version: Option<u32>,
    pub show: bool,
}

/// One row of Play: what the deck loads (`scene:<id>`) or plays as its own conductor (`show:<id>`), its words, whether
/// it is a demo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Playable {
    pub value: String,
    pub label: String,
    pub demo: bool,
    pub show: bool,
}

/// What Play lists: the sandbox toon's scenes and shows (the demos) first, then the other published scenes and shows,
/// each by name; `listed` is the API's listing (empty offline), `kept` the published scenes this deck keeps (offline
/// too). Each once.
pub fn play_list(listed: &[Listed], kept: &[Listed]) -> Vec<Playable> {
    let mut seen = std::collections::HashSet::new();
    let all: Vec<&Listed> = listed.iter().chain(kept).filter(|l| !l.id.is_empty() && seen.insert((l.show, l.id.clone()))).collect();
    let demo = |l: &Listed| l.toons.iter().any(|t| t == DEMO_TOON);
    let row = |l: &Listed| {
        let kind = if l.show { "show" } else { "scene" };
        Playable {
            value: l.version.map_or_else(|| format!("{kind}:{}", l.id), |v| format!("{kind}:{}@{v}", l.id)),
            label: if l.name.trim().is_empty() { l.id.clone() } else { l.name.clone() },
            demo: demo(l),
            show: l.show,
        }
    };
    let mut out: Vec<Playable> = all.iter().filter(|l| demo(l)).map(|l| row(l)).collect();
    out.sort_by(|a, b| a.label.cmp(&b.label));
    let mut rest: Vec<Playable> = all.iter().filter(|l| !demo(l)).map(|l| row(l)).collect();
    rest.sort_by(|a, b| a.label.cmp(&b.label));
    out.extend(rest);
    out
}

/// Play's count: `3 SCENES`, `3 SCENES · 4 SHOWS`
pub fn play_count(play: &[Playable]) -> String {
    let shows = play.iter().filter(|p| p.show).count();
    match shows {
        0 => format!("{} SCENES", play.len()),
        n => format!("{} SCENES · {n} SHOWS", play.len() - n),
    }
}

/// Where Play's listing from the API stands, for the words of an empty Play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Listing {
    /// answered (or not the deck's to fetch): an empty Play has nothing yet
    #[default]
    Answered,
    /// asked, no answer yet
    Loading,
    /// the last ask failed: asked again later, the kept copies list meanwhile
    Offline,
}

impl Listing {
    /// PLAY's value while it lists nothing
    pub fn empty_word(self) -> &'static str {
        match self {
            Listing::Answered => "NOTHING YET",
            Listing::Loading => "LOADING",
            Listing::Offline => "OFFLINE",
        }
    }
}

/// a failed listing is asked again after 2, 4, 8 … s, at most every this many seconds
pub const LIST_RETRY_MAX: f32 = 60.0;
/// an answered listing is asked again when Play opens, at most every this many seconds
pub const LIST_REFRESH: f32 = 30.0;

/// When the adapter asks the API for Play's listing (a deck may boot before its network is up): at once, then while
/// it fails with a growing pause, and again when Play opens. The adapter runs one ask at a time and keeps the last
/// good answer; times are the adapter's seconds.
#[derive(Debug, Clone, Default)]
pub struct Lister {
    off: bool,
    asking: bool,
    answered: bool,
    failures: u32,
    next: f32,
    answered_at: f32,
}

impl Lister {
    /// a check run's: never asks
    pub fn off() -> Lister {
        Lister { off: true, answered: true, ..Lister::default() }
    }

    /// whether to start an ask now; true marks it under way
    pub fn due(&mut self, now: f32) -> bool {
        if self.off || self.asking || now < self.next {
            return false;
        }
        self.asking = true;
        true
    }

    /// the ask under way came back: `ok` = the API answered (its listing is the new one)
    pub fn answered(&mut self, ok: bool, now: f32) {
        self.asking = false;
        if ok {
            self.answered = true;
            self.failures = 0;
            self.answered_at = now;
            self.next = f32::INFINITY;
        } else {
            self.failures += 1;
            self.next = now + 2f32.powi(self.failures.min(6) as i32).min(LIST_RETRY_MAX);
        }
    }

    /// Play opened: a failed listing is asked again at once, an answered one when it is older than [`LIST_REFRESH`]
    pub fn opened(&mut self, now: f32) {
        if self.off || self.asking {
            return;
        }
        let at = if self.failures > 0 { now } else { self.answered_at + LIST_REFRESH };
        self.next = self.next.min(at.max(now));
    }

    /// what an empty Play says: the last ask failed – offline; nothing answered yet – loading
    pub fn listing(&self) -> Listing {
        match (self.failures, self.answered) {
            (0, true) => Listing::Answered,
            (0, false) => Listing::Loading,
            _ => Listing::Offline,
        }
    }
}

/// A show this deck plays on its own, as its own conductor: its name, the row playing (1-based; 0: none yet), how many
/// rows it has.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Showing {
    pub name: String,
    pub row: usize,
    pub rows: usize,
}

impl Showing {
    /// the show page's line: `ROW 2 OF 3`, `NO ROW YET`
    pub fn line(&self) -> String {
        match self.row {
            0 => "NO ROW YET".into(),
            r => format!("ROW {r} OF {}", self.rows),
        }
    }

    /// GO's value: the row it fires, or `THE END` after the last
    pub fn next(&self) -> String {
        if self.row >= self.rows {
            "THE END".into()
        } else {
            format!("ROW {}", self.row + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejoin() -> Begin {
        Begin::of(Some("CHOIR"), false)
    }

    #[test]
    fn driven_by_the_conductors_scene() {
        let h = Heard { scene: true, clock: Clock::Own };
        assert!(h.driven());
        assert_eq!(stand(&Begin::Independent, &h, None, false), Stand::Driven, "the menu gives way");
        assert_eq!(stand(&rejoin(), &h, Some(true), false), Stand::Driven);
    }

    #[test]
    fn driven_by_the_conductors_clock() {
        let h = Heard { scene: false, clock: Clock::Conductor };
        assert!(h.driven());
        assert_eq!(stand(&Begin::Independent, &h, Some(true), false), Stand::Driven);
    }

    #[test]
    fn a_local_midi_clock_does_not_drive() {
        let h = Heard { scene: false, clock: Clock::Midi };
        assert!(!h.driven());
        assert_eq!(stand(&Begin::Independent, &h, None, false), Stand::Menu, "the menu stays");
        assert_eq!(stand(&Begin::Independent, &Heard::default(), None, false), Stand::Menu);
    }

    #[test]
    fn a_restarted_deck_rejoins_its_session() {
        assert_eq!(Begin::of(Some("CHOIR"), true), Begin::Rejoin("CHOIR".into()), "the session over a named scene");
        assert_eq!(Begin::of(Some(" "), false), Begin::Independent);
        assert_eq!(Begin::of(None, true), Begin::Named);
        // the conductor there or not known yet: in its session, no menu, until the snapshot drives it
        assert_eq!(stand(&rejoin(), &Heard::default(), Some(true), false), Stand::Joined);
        assert_eq!(stand(&rejoin(), &Heard::default(), None, false), Stand::Joined);
        assert_eq!(stand(&rejoin(), &Heard { scene: true, clock: Clock::Own }, Some(true), false), Stand::Driven, "the snapshot's scene");
        assert_eq!(stand(&Begin::Named, &Heard::default(), None, false), Stand::Playing);
    }

    #[test]
    fn waiting_when_the_conductor_is_gone_and_leave_goes_to_the_menu() {
        assert_eq!(stand(&rejoin(), &Heard::default(), Some(false), false), Stand::Waiting("CHOIR".into()));
        assert_eq!(waiting_line("CHOIR"), "waiting for CHOIR");
        assert_eq!(stand(&rejoin(), &Heard::default(), Some(false), true), Stand::Menu, "Leave");
        assert_eq!(stand(&rejoin(), &Heard { scene: false, clock: Clock::Midi }, Some(false), false), Stand::Waiting("CHOIR".into()), "its own MIDI clock does not end the wait");
    }

    #[test]
    fn play_lists_the_demos_first_then_the_published_scenes() {
        let l = |id: &str, name: &str, toon: &str| Listed { id: id.into(), name: name.into(), toons: vec![toon.into()], ..Listed::default() };
        let listed = [l("nave", "Nave", "catpope"), l("sandy-dunes-sandgirl", "Sandgirl", DEMO_TOON), l("sandy-dunes", "Dunes", DEMO_TOON)];
        let kept = [Listed { version: Some(3), ..l("nave", "Nave", "catpope") }, Listed { version: Some(2), ..l("vigil", "", "catpope") }];
        let p = play_list(&listed, &kept);
        let words: Vec<(&str, &str, bool)> = p.iter().map(|p| (p.value.as_str(), p.label.as_str(), p.demo)).collect();
        assert_eq!(
            words,
            [("scene:sandy-dunes", "Dunes", true), ("scene:sandy-dunes-sandgirl", "Sandgirl", true), ("scene:nave", "Nave", false), ("scene:vigil@2", "vigil", false)]
        );
        assert!(play_list(&[], &[]).is_empty(), "offline with nothing kept: an empty list");
        assert_eq!(play_count(&p), "4 SCENES");
        assert_eq!(play_list(&[], &kept)[0].value, "scene:nave@3", "offline: the kept copy plays without asking");
    }

    #[test]
    fn play_lists_the_sandbox_shows_next_to_its_scenes_first() {
        let l = |id: &str, toon: &str, show: bool| Listed { id: id.into(), name: id.into(), toons: vec![toon.into()], show, ..Listed::default() };
        let listed = [l("nave", "catpope", false), l("demo-new-scene", DEMO_TOON, true), l("sandy-dunes", DEMO_TOON, false), l("choir", "catpope", true), l("demo-far-nodes", DEMO_TOON, true)];
        let p = play_list(&listed, &[]);
        let values: Vec<&str> = p.iter().map(|p| p.value.as_str()).collect();
        assert_eq!(values, ["show:demo-far-nodes", "show:demo-new-scene", "scene:sandy-dunes", "show:choir", "scene:nave"]);
        assert!(p[0].show && p[0].demo && !p[2].show && !p[3].demo);
        assert_eq!(play_count(&p), "2 SCENES · 3 SHOWS");
        // a show and a scene of the same id are two rows
        assert_eq!(play_list(&[l("x", DEMO_TOON, true), l("x", DEMO_TOON, false)], &[]).len(), 2);
    }

    #[test]
    fn a_deck_that_boots_offline_asks_again_until_the_listing_comes() {
        let mut l = Lister::default();
        assert_eq!(l.listing(), Listing::Loading);
        assert!(l.due(0.0), "asked at once");
        assert!(!l.due(0.5), "one ask at a time");
        // offline: asked again after 2, 4, 8 … s, never more than a minute apart
        l.answered(false, 1.0);
        assert_eq!(l.listing(), Listing::Offline);
        assert!(!l.due(2.9) && l.due(3.0));
        l.answered(false, 3.0);
        assert!(!l.due(6.9) && l.due(7.0));
        for _ in 0..10 {
            l.answered(false, 100.0);
            assert!(l.due(100.0 + LIST_RETRY_MAX));
        }
        // Play opened while offline: at once
        l.answered(false, 200.0);
        l.opened(201.0);
        assert!(l.due(201.0));
        // the network is up: listed, no more asks until Play opens after the refresh time
        l.answered(true, 202.0);
        assert_eq!(l.listing(), Listing::Answered);
        assert!(!l.due(10_000.0));
        l.opened(210.0);
        assert!(!l.due(231.9) && l.due(232.0));
        // a refresh that fails keeps the list's words (the adapter keeps the last answer) but says offline when empty
        l.answered(false, 233.0);
        assert_eq!(l.listing().empty_word(), "OFFLINE");
    }

    #[test]
    fn a_check_run_never_asks() {
        let mut l = Lister::off();
        l.opened(5.0);
        assert!(!l.due(0.0) && !l.due(1e6));
        assert_eq!(l.listing(), Listing::Answered);
    }

    #[test]
    fn a_show_s_lines() {
        let s = Showing { name: "demo-new-scene".into(), row: 0, rows: 3 };
        assert_eq!((s.line(), s.next()), ("NO ROW YET".into(), "ROW 1".into()));
        let s = Showing { row: 3, ..s };
        assert_eq!((s.line(), s.next()), ("ROW 3 OF 3".into(), "THE END".into()));
    }
}
