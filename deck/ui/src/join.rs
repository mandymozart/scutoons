//! JOIN A SHOW: the code page of the start menu – a session code typed with keys or picked on the on-screen keys (a
//! gamepad, a finger), Enter joins it on the relay in the deck's role. The rules and the words are here; the adapter
//! starts the join when the page asks and tells how the relay answered ([`Reply`] → [`Join::settle`]).
//! An unknown code is no refusal: the relay opens a session for any code, so a code nobody conducts ends on the waiting
//! page ("waiting for <code>"); a refusal (a full session, a role's token) or silence says why here.
use crate::menu::{Hit, Input};
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::live::hud::{self as look, FINGER, GLYPH, ROW};

/// a code's longest
pub const MAX: usize = 24;
/// the on-screen keys: the characters the API's codes are made of (LiveSessions.php); then DEL and JOIN
pub const KEYS: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const COLS: usize = 8;
const DEL: usize = KEYS.len();
const GO: usize = DEL + 1;
/// seconds without an answer from the relay
pub const PATIENCE: f32 = 12.0;

/// A typed key from a keyboard (the adapter maps its events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    Char(char),
    Delete,
}

/// What the page asks of the menu after an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Stay,
    /// Esc: one page back (a join under way is given up)
    Back { joining: bool },
    /// Enter on a code: join it
    Join(String),
}

/// What the relay client says of the join, as plain values.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reply {
    /// the session the client is in, and whether the relay welcomed it
    pub code: String,
    pub joined: bool,
    /// the relay's last refusal (`full: session full`), if any
    pub refused: Option<String>,
    /// the connection's own line (what it is doing: connecting, retrying)
    pub link: String,
}

/// How a join under way stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Waiting,
    /// the relay welcomed the deck: the page is done
    In(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum State {
    #[default]
    Typing,
    /// asked for `code` at `since` (the first settle's time)
    Joining { code: String, since: Option<f32> },
    /// why the last try did not join
    Said(String),
}

/// The page: the code, where the keys are (none: on the field – typed characters and Enter are its own), the state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Join {
    code: String,
    at: Option<usize>,
    state: State,
}

/// the characters a code is made of; a typed letter is upper case (the page and the relay's key are the same)
fn allowed(c: char) -> Option<char> {
    (c.is_ascii_alphanumeric() || c == '-' || c == '_').then(|| c.to_ascii_uppercase())
}

/// the relay's address for session `code`: `relay` as configured (or the hosted one) without its own session and query
pub fn relay_url(relay: &str, code: &str) -> String {
    let base = relay.split('?').next().unwrap_or(relay).trim_end_matches('/');
    let base = base.find("/live/").map_or(base, |i| &base[..i]);
    format!("{base}/live/{code}")
}

impl Join {
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn joining(&self) -> bool {
        matches!(self.state, State::Joining { .. })
    }

    /// the page opens empty on the field
    pub fn open(&mut self) {
        *self = Join::default();
    }

    /// the code, or why there is none to join
    pub fn ask(&mut self) -> Step {
        if self.code.is_empty() {
            self.state = State::Said("TYPE THE SESSION CODE".into());
            return Step::Stay;
        }
        self.state = State::Joining { code: self.code.clone(), since: None };
        Step::Join(self.code.clone())
    }

    /// a key of a keyboard (while a join is under way the page is still)
    pub fn typed(&mut self, t: Typed) -> bool {
        if self.joining() {
            return true;
        }
        self.at = None;
        self.state = State::Typing;
        match t {
            Typed::Delete => {
                self.code.pop();
            }
            Typed::Char(c) => {
                if let (Some(c), true) = (allowed(c), self.code.chars().count() < MAX) {
                    self.code.push(c);
                }
            }
        }
        true
    }

    fn key(&mut self, k: usize) -> Step {
        self.at = Some(k);
        match k {
            DEL => {
                self.typed(Typed::Delete);
                self.at = Some(k);
                Step::Stay
            }
            GO => self.ask(),
            _ => {
                if let Some(c) = KEYS.chars().nth(k) {
                    self.typed(Typed::Char(c));
                }
                self.at = Some(k);
                Step::Stay
            }
        }
    }

    /// the key a step up / down / left / right of `k`
    fn neighbour(k: usize, input: Input) -> Option<usize> {
        let (row, col) = if k >= DEL { (KEYS.len() / COLS, (k - DEL) * COLS / 2) } else { (k / COLS, k % COLS) };
        let rows = KEYS.len() / COLS;
        let pick = |row: usize, col: usize| if row >= rows { DEL + usize::from(col >= COLS / 2) } else { row * COLS + col };
        match input {
            Input::Up => row.checked_sub(1).map(|r| pick(r, col)),
            Input::Down => (row < rows).then(|| pick(row + 1, col)),
            Input::Left if k < DEL => (col > 0).then(|| k - 1),
            Input::Right if k < DEL => (col + 1 < COLS).then(|| k + 1),
            Input::Left => (k == GO).then_some(DEL),
            Input::Right => (k == DEL).then_some(GO),
            _ => None,
        }
    }

    /// One input; `press` = the key the pointer is on (a press there), `hover` the same for a move
    pub fn input(&mut self, i: Input, on: Option<Hit>) -> Step {
        if self.joining() {
            return match i {
                Input::Back | Input::Secondary | Input::Open => Step::Back { joining: true },
                Input::Press(..) if on == Some(Hit::Back) => Step::Back { joining: true },
                _ => Step::Stay,
            };
        }
        match i {
            Input::Back | Input::Secondary | Input::Open => Step::Back { joining: false },
            Input::Enter => match self.at {
                None => self.ask(),
                Some(k) => self.key(k),
            },
            Input::Up | Input::Down | Input::Left | Input::Right => {
                self.at = match (self.at, i) {
                    (None, Input::Down) => Some(0),
                    (None, _) => None,
                    (Some(k), Input::Up) if k < COLS => None,
                    (Some(k), _) => Self::neighbour(k, i).or(Some(k)),
                };
                Step::Stay
            }
            Input::Point(..) => {
                if let Some(Hit::Row(k)) = on {
                    self.at = Some(k);
                }
                Step::Stay
            }
            Input::Press(..) => match on {
                Some(Hit::Row(k)) => self.key(k),
                Some(Hit::Back) => Step::Back { joining: false },
                None => Step::Stay,
            },
        }
    }

    /// The relay's answer to a join under way, `now` in seconds; the page keeps what it says.
    pub fn settle(&mut self, now: f32, r: &Reply) -> Outcome {
        let State::Joining { code, since } = &mut self.state else { return Outcome::Waiting };
        let since = *since.get_or_insert(now);
        let why = if r.joined && r.code == *code {
            let code = std::mem::take(code);
            *self = Join::default();
            return Outcome::In(code);
        } else if let Some(why) = &r.refused {
            format!("THE RELAY SAYS {}", why.to_uppercase())
        } else if now - since >= PATIENCE {
            let link = if r.link.is_empty() { String::new() } else { format!(": {}", r.link.to_uppercase()) };
            format!("NO ANSWER FROM THE RELAY{link}")
        } else {
            return Outcome::Waiting;
        };
        self.state = State::Said(why.clone());
        Outcome::Failed(why)
    }

    /// the join could not start: why, on the page
    pub fn fail(&mut self, why: String) {
        self.state = State::Said(why);
    }

    /// the line under the field
    fn said(&self) -> (String, bool) {
        match &self.state {
            State::Typing => ("ENTER JOINS THE SHOW OF THIS CODE".into(), true),
            State::Joining { code, .. } => (format!("JOINING {code} ..."), false),
            State::Said(why) => (why.clone(), false),
        }
    }

    /// The body from `y` into `d`: the field, what it says, the keys (each a place to press: `Hit::Row(key)`); its height.
    pub fn draw(&self, d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), finger: bool, places: &mut Vec<(Rect, Hit)>) -> f32 {
        let y0 = y;
        let mut y = y;
        let cursor = if self.at.is_none() && !self.joining() { "_" } else { "" };
        y += look::line(d, t, (x, y, w), &format!("CODE  {}{cursor}", self.code), "", false);
        let (said, muted) = self.said();
        for line in crate::wrap(&said, look::cols(d, w)).iter().take(2) {
            y += look::line(d, t, (x, y, w), line, "", muted);
        }
        y += look::rule(d, t, (x, y, w));
        let keys: Vec<(usize, String)> = KEYS.chars().enumerate().map(|(i, c)| (i, c.to_string())).chain([(DEL, "DEL".into()), (GO, "JOIN".into())]).collect();
        let (cell, high) = (w / COLS as f32, if finger { FINGER } else { ROW });
        for (row, line) in keys.chunks(COLS).enumerate() {
            let across = if line.len() < COLS { w / line.len() as f32 } else { cell };
            for (n, (k, label)) in line.iter().enumerate() {
                let r = Rect { x: x + n as f32 * across, y: y + row as f32 * high, w: across, h: high };
                if self.at == Some(*k) {
                    d.rect(r.x, r.y, r.w, r.h, t.color.selected);
                }
                look::centred(d, t, (r.x, r.y + (high - ROW) * 0.5 + 1.0, r.w), GLYPH, label, false);
                places.push((r, Hit::Row(*k)));
            }
        }
        y += (KEYS.len() / COLS + 1) as f32 * high;
        y += look::rule(d, t, (x, y, w));
        if !finger {
            y += look::line(d, t, (x, y, w), "TYPE  ARROWS KEYS  ENTER JOIN  ESC BACK", "", true);
        }
        y - y0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_in(j: &mut Join, s: &str) {
        s.chars().for_each(|c| {
            j.typed(Typed::Char(c));
        });
    }

    #[test]
    fn a_typed_code_is_upper_case_and_joins_on_enter() {
        let mut j = Join::default();
        type_in(&mut j, "ch/o ir-2");
        assert_eq!(j.code(), "CHOIR-2", "a slash and a space are not a code's");
        assert_eq!(j.input(Input::Enter, None), Step::Join("CHOIR-2".into()));
        assert!(j.joining());
    }

    #[test]
    fn the_on_screen_keys_type_delete_and_join() {
        let mut j = Join::default();
        assert_eq!(j.input(Input::Down, None), Step::Stay, "off the field onto the keys: A");
        j.input(Input::Enter, None);
        j.input(Input::Right, None);
        j.input(Input::Enter, None);
        assert_eq!(j.code(), "AB");
        // down to the last row: DEL, then JOIN beside it
        (0..4).for_each(|_| {
            j.input(Input::Down, None);
        });
        j.input(Input::Enter, None);
        assert_eq!(j.code(), "A", "DEL");
        j.input(Input::Right, None);
        assert_eq!(j.input(Input::Enter, None), Step::Join("A".into()), "JOIN");
        // a press on a key is the same as Enter on it
        let mut j = Join::default();
        assert_eq!(j.input(Input::Press(0.0, 0.0), Some(Hit::Row(3))), Step::Stay);
        assert_eq!(j.code(), "D");
        assert_eq!(j.input(Input::Press(0.0, 0.0), Some(Hit::Row(GO))), Step::Join("D".into()));
    }

    #[test]
    fn up_from_the_top_row_is_the_field_again() {
        let mut j = Join::default();
        j.input(Input::Down, None);
        j.input(Input::Up, None);
        type_in(&mut j, "x");
        assert_eq!(j.code(), "X");
        assert_eq!(j.input(Input::Enter, None), Step::Join("X".into()), "on the field Enter joins");
    }

    #[test]
    fn an_empty_code_says_so_and_esc_returns() {
        let mut j = Join::default();
        assert_eq!(j.input(Input::Enter, None), Step::Stay);
        assert_eq!(j.said().0, "TYPE THE SESSION CODE");
        assert_eq!(j.input(Input::Back, None), Step::Back { joining: false });
    }

    #[test]
    fn the_relays_refusal_is_said_and_the_page_can_try_again() {
        let mut j = Join::default();
        type_in(&mut j, "choir");
        j.input(Input::Enter, None);
        assert_eq!(j.settle(1.0, &Reply::default()), Outcome::Waiting);
        let refused = Reply { refused: Some("full: session full".into()), ..Reply::default() };
        assert_eq!(j.settle(2.0, &refused), Outcome::Failed("THE RELAY SAYS FULL: SESSION FULL".into()));
        assert!(!j.joining());
        assert_eq!((j.code(), j.said().0.as_str()), ("CHOIR", "THE RELAY SAYS FULL: SESSION FULL"));
        j.typed(Typed::Delete);
        assert_eq!(j.said().0, "ENTER JOINS THE SHOW OF THIS CODE", "typing again clears it");
    }

    #[test]
    fn no_answer_in_time_says_so_and_the_welcome_ends_the_page() {
        let mut j = Join::default();
        type_in(&mut j, "CHOIR");
        j.input(Input::Enter, None);
        let down = Reply { link: "connecting".into(), ..Reply::default() };
        assert_eq!(j.settle(10.0, &down), Outcome::Waiting);
        assert_eq!(j.settle(10.0 + PATIENCE, &down), Outcome::Failed("NO ANSWER FROM THE RELAY: CONNECTING".into()));
        j.input(Input::Enter, None);
        // another session's welcome is not this one's
        assert_eq!(j.settle(30.0, &Reply { code: "OTHER".into(), joined: true, ..Reply::default() }), Outcome::Waiting);
        assert_eq!(j.settle(31.0, &Reply { code: "CHOIR".into(), joined: true, ..Reply::default() }), Outcome::In("CHOIR".into()));
        assert_eq!(j, Join::default(), "the page is empty again");
    }

    #[test]
    fn esc_while_joining_gives_the_join_up() {
        let mut j = Join::default();
        type_in(&mut j, "A");
        j.input(Input::Enter, None);
        assert_eq!(j.input(Input::Up, None), Step::Stay);
        assert_eq!(j.input(Input::Back, None), Step::Back { joining: true });
    }

    #[test]
    fn the_relay_url_keeps_the_configured_relay_and_swaps_the_session() {
        assert_eq!(relay_url("wss://live.scutoons.com/live/CATH", "A7K2"), "wss://live.scutoons.com/live/A7K2");
        assert_eq!(relay_url("ws://127.0.0.1:7070", "A7K2"), "ws://127.0.0.1:7070/live/A7K2");
        assert_eq!(relay_url("ws://h:7700/live/X?quic=1", "A"), "ws://h:7700/live/A");
    }
}
