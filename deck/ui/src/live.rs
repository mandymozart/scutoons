//! The Live page (#478, F6; owner 2026-11-10 [role]): inside the deck, native and browser alike, the session and its state,
//! the roles this deck may take with how many are in each, "no role", the name as free text. Enter or a tap takes a role at
//! once – a deck may change its role at any time; a lock from above holds it (the cascade of 2026-11-02 [venue]): the page
//! says so and takes nothing. The roles are the conductor's (a stage outlet of its show, deck-show), the relay offers each
//! client only those it may take (deck-relay); what a deck has of them is its relay client's (deck-space relay.rs
//! `Client::live`). Plain data in, the page's words out; the adapter maps its keys and taps and does what [`Do`] says.

/// One role this deck may take: its id (the outlet), its name, how many are in it now.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Offer {
    pub id: String,
    pub label: String,
    pub here: u32,
}

/// What the deck has of its session.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Session {
    /// the session's code (`CHOIR`)
    pub code: String,
    /// the relay client's line (joining, connected as, offline …)
    pub link: String,
    pub joined: bool,
    pub offers: Vec<Offer>,
    /// the role it is in (none: no role – it stays in the session on the idle scene)
    pub role: Option<String>,
    /// the name it says (free text, the conductor sees `role: name`)
    pub name: String,
    /// the conductor put it there with a lock from above: it cannot leave
    pub locked: bool,
}

impl Session {
    /// the role's name for people: the offer's label, else its id
    pub fn label(&self, id: &str) -> String {
        self.offers.iter().find(|o| o.id == id).map_or_else(|| id.to_string(), |o| o.label.clone())
    }
}

/// the word for a deck in no role (the page's line, the HUD's and the conductor's `role: name`)
pub const NO_ROLE: &str = "no role";

/// `role: name` – what the HUD line and the conductor say (owner 2026-11-10: "Rolle, Doppelpunkt, eingetragener Name");
/// without a name, the role alone
pub fn role_name(role: Option<&str>, name: &str) -> String {
    let role = role.unwrap_or(NO_ROLE);
    match name.trim() {
        "" => role.to_string(),
        n => format!("{role}: {n}"),
    }
}

/// this deck's `role: name`, the role by its label
pub fn who(s: &Session) -> String {
    role_name(s.role.as_deref().map(|r| s.label(r)).as_deref(), &s.name)
}

/// The HUD line of a deck in a session: `LIVE TENOR: ANNA` (upper case, font8x8)
pub fn hud_line(s: &Session) -> String {
    format!("LIVE {}", who(s)).to_uppercase()
}

/// A key the page takes (the adapter maps its own: arrows, Enter, Backspace, a typed character).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
    Back,
    Char(char),
}

/// What the adapter does for the page: take a role (none: no role), say a new name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Do {
    Take(Option<String>),
    Name(String),
}

/// the name's longest (the relay trims a nick to 24 characters)
pub const NAME_MAX: usize = 24;
const VISIBLE: usize = 8;
const WIDTH: usize = crate::COLS;
/// the lines before the entries: the title, the session
const HEAD: usize = 2;

/// One line of the list.
#[derive(Debug, Clone, PartialEq)]
enum Entry {
    Role(Offer),
    None,
    Name,
}

/// The page: open or not, the cursor, the name being typed, the last result.
#[derive(Debug, Clone, Default)]
pub struct Page {
    pub open: bool,
    cursor: usize,
    /// the name being typed (none: the session's, as it is)
    name: Option<String>,
    /// what the last key did (taken · locked · the name said)
    pub said: String,
}

fn entries(s: &Session) -> Vec<Entry> {
    let mut out: Vec<Entry> = s.offers.iter().cloned().map(Entry::Role).collect();
    // a role the conductor put it in that is not offered to it still shows (it sits there)
    if let Some(r) = s.role.as_ref().filter(|r| !s.offers.iter().any(|o| &o.id == *r)) {
        out.insert(0, Entry::Role(Offer { id: r.clone(), label: r.clone(), here: 0 }));
    }
    out.extend([Entry::None, Entry::Name]);
    out
}

impl Page {
    /// Opens on the role it is in, the name as it is.
    pub fn open(&mut self, s: &Session) {
        let list = entries(s);
        self.cursor = list.iter().position(|e| matches!((e, &s.role), (Entry::Role(o), Some(r)) if &o.id == r) || (matches!(e, Entry::None) && s.role.is_none())).unwrap_or(0);
        self.name = None;
        self.said.clear();
        self.open = true;
    }

    fn typed(&self, s: &Session) -> String {
        self.name.clone().unwrap_or_else(|| s.name.clone())
    }

    /// whether the cursor is on the name (typed characters go there)
    pub fn on_name(&self, s: &Session) -> bool {
        matches!(entries(s).get(self.cursor), Some(Entry::Name))
    }

    /// One key; Enter on a role (or "no role") takes it at once, on the name says it.
    pub fn key(&mut self, k: Key, s: &Session) -> Option<Do> {
        let list = entries(s);
        self.cursor = self.cursor.min(list.len() - 1);
        match k {
            Key::Up => self.cursor = self.cursor.saturating_sub(1),
            Key::Down => self.cursor = (self.cursor + 1).min(list.len() - 1),
            Key::Back if self.on_name(s) => {
                self.name.get_or_insert_with(|| s.name.clone()).pop();
            }
            Key::Char(c) if self.on_name(s) && !c.is_control() && self.typed(s).chars().count() < NAME_MAX => self.name.get_or_insert_with(|| s.name.clone()).push(c),
            Key::Enter => return self.pick(&list[self.cursor], s),
            Key::Back | Key::Char(_) => {}
        }
        None
    }

    /// A tap on the page's HUD line `line` (0 = the title): the entry there is chosen as with Enter.
    pub fn tap(&mut self, line: usize, s: &Session) -> Option<Do> {
        let list = entries(s);
        let i = (line.checked_sub(HEAD)?) + self.start(list.len());
        let e = list.get(i)?.clone();
        self.cursor = i;
        self.pick(&e, s)
    }

    fn pick(&mut self, e: &Entry, s: &Session) -> Option<Do> {
        let take = match e {
            Entry::Name => {
                let name: String = self.typed(s).trim().chars().take(NAME_MAX).collect();
                self.said = format!("you are {}", role_name(s.role.as_deref().map(|r| s.label(r)).as_deref(), &name));
                return Some(Do::Name(name));
            }
            Entry::Role(o) => Some(o.id.clone()),
            Entry::None => None,
        };
        if s.locked {
            // the page's own line says it
            self.said.clear();
            return None;
        }
        if !s.joined {
            self.said = "not in a session".into();
            return None;
        }
        self.said = format!("taking {}", take.as_deref().map(|r| s.label(r)).unwrap_or(NO_ROLE.into()));
        Some(Do::Take(take))
    }

    fn start(&self, n: usize) -> usize {
        (self.cursor + 1).saturating_sub(VISIBLE).min(n.saturating_sub(VISIBLE))
    }

    /// The page as HUD lines (≤ 13 × 39), the role it is in starred; also the cursor's line.
    pub fn lines(&self, s: &Session) -> (Vec<String>, usize) {
        let list = entries(s);
        let cursor = self.cursor.min(list.len() - 1);
        let link = if s.link.is_empty() { "NOT IN A SESSION".to_string() } else { s.link.to_uppercase() };
        let mut out = vec!["LIVE  F6 CLOSE  ENTER=TAKE".to_string(), link];
        let start = self.start(list.len());
        let mut at = HEAD;
        for (i, e) in list.iter().enumerate().skip(start).take(VISIBLE) {
            if i == cursor {
                at = out.len();
            }
            let here = matches!((e, &s.role), (Entry::Role(o), Some(r)) if &o.id == r) || (matches!(e, Entry::None) && s.role.is_none());
            let (what, right) = match e {
                Entry::Role(o) => (o.label.clone(), if o.here == 0 { "NOBODY YET".to_string() } else { format!("{} HERE", o.here) }),
                Entry::None => (NO_ROLE.to_string(), "IDLE SCENE".to_string()),
                Entry::Name => {
                    let typing = if i == cursor { "_" } else { "" };
                    (format!("name: {}{typing}", self.typed(s)), String::new())
                }
            };
            let left = format!("{}{}{}", if i == cursor { '>' } else { ' ' }, if here { '*' } else { ' ' }, what);
            let pad = WIDTH.saturating_sub(left.chars().count() + right.chars().count()).max(1);
            let line = format!("{left}{}{right}", " ".repeat(pad));
            out.push(line.to_uppercase().chars().take(WIDTH).collect());
        }
        if s.locked {
            out.push("LOCKED BY THE CONDUCTOR".into());
        }
        if !self.said.is_empty() {
            out.extend(crate::wrap(&self.said.to_uppercase(), WIDTH).into_iter().take(1));
        }
        out.truncate(crate::LINES);
        for l in out.iter_mut() {
            *l = l.chars().take(WIDTH).collect();
        }
        (out, at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choir() -> Session {
        let offer = |id: &str, here| Offer { id: id.into(), label: id.into(), here };
        Session {
            code: "CHOIR".into(),
            link: "OUTPUT C3 OF TENOR @ CHOIR".into(),
            joined: true,
            offers: vec![offer("tenor", 2), offer("soprano", 1), offer("alto", 0)],
            role: Some("tenor".into()),
            name: "Anna".into(),
            locked: false,
        }
    }

    #[test]
    fn the_page_lists_the_roles_no_role_and_the_name() {
        let s = choir();
        let mut p = Page::default();
        p.open(&s);
        let (lines, cursor) = p.lines(&s);
        assert_eq!(lines[0], "LIVE  F6 CLOSE  ENTER=TAKE");
        assert!(lines[2].starts_with(">*TENOR ") && lines[2].ends_with(" 2 HERE") && lines[2].chars().count() == crate::COLS, "{lines:?}");
        assert_eq!(cursor, 2, "the cursor on the role it is in");
        assert!(lines[4].ends_with("NOBODY YET") && lines[5].contains("NO ROLE") && lines[5].ends_with("IDLE SCENE"));
        assert_eq!(lines[6].trim_end(), "  NAME: ANNA");
        assert!(lines.iter().all(|l| l.chars().count() <= crate::COLS) && lines.len() <= crate::LINES);
        assert_eq!(hud_line(&s), "LIVE TENOR: ANNA");
        assert_eq!(role_name(None, "phone-7"), "no role: phone-7");
        assert_eq!(role_name(Some("click"), " "), "click");
    }

    #[test]
    fn enter_or_a_tap_takes_a_role_at_once_and_no_role() {
        let s = choir();
        let mut p = Page::default();
        p.open(&s);
        p.key(Key::Down, &s);
        assert_eq!(p.key(Key::Enter, &s), Some(Do::Take(Some("soprano".into()))));
        assert_eq!(p.tap(5, &s), Some(Do::Take(None)), "line 5 is no role");
        assert_eq!(p.lines(&s).0.last().unwrap(), "TAKING NO ROLE");
        assert_eq!(p.tap(0, &s), None, "the title takes nothing");
        assert_eq!(p.tap(40, &s), None);
    }

    #[test]
    fn the_name_is_typed_and_said() {
        let s = choir();
        let mut p = Page::default();
        p.open(&s);
        for _ in 0..9 {
            p.key(Key::Down, &s);
        }
        assert!(p.on_name(&s));
        for _ in 0..4 {
            p.key(Key::Back, &s);
        }
        for c in "Ben the long-winded tenor".chars() {
            p.key(Key::Char(c), &s);
        }
        assert_eq!(p.key(Key::Enter, &s), Some(Do::Name("Ben the long-winded teno".into())), "24 characters");
        p.key(Key::Up, &s);
        p.key(Key::Char('x'), &s);
        assert!(p.lines(&s).0.iter().all(|l| !l.contains("TENOX")), "a character off the name line types nothing");
    }

    #[test]
    fn a_lock_from_above_holds_the_role() {
        let mut s = choir();
        s.locked = true;
        let mut p = Page::default();
        p.open(&s);
        p.key(Key::Down, &s);
        assert_eq!(p.key(Key::Enter, &s), None);
        let (lines, _) = p.lines(&s);
        assert!(lines.contains(&"LOCKED BY THE CONDUCTOR".to_string()));
        assert_eq!(lines.last().unwrap(), "LOCKED BY THE CONDUCTOR");
        // the name may still change
        for _ in 0..4 {
            p.key(Key::Down, &s);
        }
        assert!(matches!(p.key(Key::Enter, &s), Some(Do::Name(_))));
    }

    #[test]
    fn a_role_it_was_put_in_shows_and_outside_a_session_nothing_is_taken() {
        let mut s = choir();
        s.role = Some("click".into());
        let mut p = Page::default();
        p.open(&s);
        assert!(p.lines(&s).0[2].starts_with(">*CLICK"));
        s.joined = false;
        s.link.clear();
        assert_eq!(p.key(Key::Enter, &s), None);
        assert_eq!(p.lines(&s).0[1], "NOT IN A SESSION");
    }
}
