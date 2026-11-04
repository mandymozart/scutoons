//! When the outlet composes its cell again (#391; was the composer thread's loop in deck-native's live/mod.rs): a new cell,
//! panic black, the show's slice changing under surtitles, new media while a cell waits for it, a resize, the scroll's hold.
//! A sans-IO state machine: the inputs (`route::Input`), the plane's size and a clock go in (`now`: the time since the
//! adapter's start – a thread reads `Instant`, a browser its animation clock), and out comes whether to compose a fresh
//! cell or step the scroll. The adapter keeps the thread, the channel, the composer itself (`compose::Composer`) and the clock.
use crate::frame::SpaceTarget;
use crate::plan::{plan, Plan};
use crate::route::Input;
use protocol::{Cell, CellMode};
use std::time::Duration;

/// how long a scroll layer holds a slice (the chapter node's default hold)
pub const SCROLL_HOLD: Duration = Duration::from_secs(6);
/// the longest an adapter waits for an input before it looks at the clock again
pub const POLL: Duration = Duration::from_millis(200);

/// What to compose: a new cell (`fresh`: the plan from its start) or the next step of the one on (`scroll_step`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compose {
    pub fresh: bool,
}

/// What a composed cell leaves for the adapter: the space node's new target (None: unchanged) and the HUD's cell line.
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    pub space: Option<SpaceTarget>,
    pub cell: String,
}

/// The composer's decisions: the cell on, its plan, the show's slice, the scroll's step and when it is due.
#[derive(Debug, Clone)]
pub struct Recompose {
    locale: String,
    cell: bool,
    planned: Plan,
    slice: Option<usize>,
    scroll: usize,
    next_step: Option<Duration>,
    waiting_for_media: bool,
    size: (u32, u32),
    space_sent: Option<SpaceTarget>,
}

impl Recompose {
    /// for an outlet reading cells in `locale`, its plane `size` pixels
    pub fn new(locale: &str, size: (u32, u32)) -> Recompose {
        Recompose { locale: locale.to_owned(), cell: false, planned: Plan::default(), slice: None, scroll: 0, next_step: None, waiting_for_media: false, size, space_sent: None }
    }

    /// how long to wait for the next input at `now`: until the scroll is due, at most `POLL`
    pub fn wait(&self, now: Duration) -> Duration {
        self.next_step.map_or(POLL, |t| t.saturating_sub(now).min(POLL))
    }

    /// One turn: `input` (None: the wait ran out), the plane's `size` now and `now`; `carried` says which modules this
    /// build carries (the plan marks the others as problems). Some = compose now, with `plan()`, `slice()`, `scroll()`
    /// and `size()` as the composer reads them.
    pub fn step(&mut self, input: Option<Input>, size: (u32, u32), now: Duration, carried: impl Fn(&str) -> bool) -> Option<Compose> {
        let (mut fresh, mut step) = (false, false);
        match input {
            Some(Input::Cell(c)) => {
                self.planned = plan(&c, &self.locale, carried);
                self.cell = true;
                self.scroll = 0;
                fresh = true;
            }
            Some(Input::Black) => {
                let black = Cell { mode: CellMode::Black, layers: None, of: None, send: None };
                self.planned = plan(&black, &self.locale, carried);
                self.cell = true;
                fresh = true;
            }
            Some(Input::Slice(s)) => {
                let changed = self.slice != Some(s);
                self.slice = Some(s);
                step = changed && self.planned.follows_the_slice();
            }
            Some(Input::Media) => step = self.waiting_for_media,
            None => {}
        }
        if size != self.size && self.cell {
            self.size = size;
            step = true;
        }
        if self.next_step.is_some_and(|t| now >= t) {
            self.scroll += 1;
            step = true;
        }
        (self.cell && (fresh || step)).then_some(Compose { fresh })
    }

    /// The cell was composed (`took_ms` of work): the scroll's next step is due `SCROLL_HOLD` after `now`, a cell that
    /// lacks media waits for it, the space node gets a target only when it changed, the HUD gets its cell line.
    pub fn composed(&mut self, now: Duration, took_ms: f32, backdrop: bool, problems: &[String], space: &Option<SpaceTarget>) -> Done {
        self.next_step = self.planned.scrolls().then_some(now + SCROLL_HOLD);
        self.waiting_for_media = problems.iter().any(|p| p.contains("not preloaded"));
        let changed = if *space != self.space_sent { space.clone() } else { None };
        self.space_sent = space.clone();
        Done { space: changed, cell: self.cell_line(took_ms, backdrop) }
    }

    /// `CELL` n LAYERS [+ SPACE] [SCROLL k] t MS – what the HUD says of the cell on
    fn cell_line(&self, took_ms: f32, backdrop: bool) -> String {
        format!(
            "{} LAYERS{}{} {:.0} MS",
            self.planned.layers.len(),
            if backdrop { " + SPACE" } else { "" },
            if self.planned.scrolls() { format!(" SCROLL {}", self.scroll) } else { String::new() },
            took_ms
        )
    }

    /// the cell on, as this deck plays it
    pub fn plan(&self) -> &Plan {
        &self.planned
    }

    /// the show's slice (None: none seen yet)
    pub fn slice(&self) -> Option<usize> {
        self.slice
    }

    /// the scroll's step (0 at a new cell)
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// the plane's size the cell is composed at
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: (u32, u32) = (1920, 1080);
    const T0: Duration = Duration::from_secs(100);

    fn cell(json: &str) -> Input {
        Input::Cell(serde_json::from_str(json).unwrap())
    }

    fn text() -> Input {
        cell(r#"{"mode":"own","layers":[{"id":"t","src":{"kind":"text","p":{"text":{"en":"hi"},"size":5}}}]}"#)
    }

    fn scroll() -> Input {
        cell(r#"{"mode":"own","layers":[{"id":"s","src":{"kind":"scroll","p":{"toon":"catpope","chapter":1}}}]}"#)
    }

    fn surtitles() -> Input {
        cell(r#"{"mode":"own","layers":[{"id":"t","src":{"kind":"surtitles","p":{"toon":"catpope","chapter":1,"locale":"en"}}}]}"#)
    }

    fn machine() -> Recompose {
        Recompose::new("en", SIZE)
    }

    fn turn(m: &mut Recompose, input: Option<Input>, at: Duration) -> Option<Compose> {
        m.step(input, SIZE, at, |_| true)
    }

    /// the cell is composed: the adapter tells the machine
    fn composed(m: &mut Recompose, at: Duration) -> Done {
        m.composed(at, 12.4, false, &[], &None)
    }

    #[test]
    fn nothing_is_composed_before_a_cell_and_a_new_cell_or_black_composes_fresh() {
        let mut m = machine();
        assert_eq!(turn(&mut m, None, T0), None);
        assert_eq!(turn(&mut m, Some(Input::Slice(3)), T0), None, "a slice without a cell");
        assert_eq!(turn(&mut m, Some(Input::Media), T0), None);
        assert_eq!(turn(&mut m, Some(text()), T0), Some(Compose { fresh: true }));
        assert_eq!((m.plan().layers.len(), m.slice(), m.scroll()), (1, Some(3), 0));
        assert_eq!(turn(&mut m, None, T0 + Duration::from_secs(1)), None, "idle");
        assert_eq!(turn(&mut m, Some(Input::Black), T0), Some(Compose { fresh: true }));
        assert!(m.plan().layers.is_empty());
    }

    #[test]
    fn surtitles_follow_the_slice_only_when_it_changes() {
        let mut m = machine();
        assert_eq!(turn(&mut m, Some(surtitles()), T0), Some(Compose { fresh: true }));
        composed(&mut m, T0);
        assert_eq!(turn(&mut m, Some(Input::Slice(4)), T0), Some(Compose { fresh: false }));
        assert_eq!(turn(&mut m, Some(Input::Slice(4)), T0), None, "the same slice again");
        assert_eq!(turn(&mut m, Some(Input::Slice(5)), T0), Some(Compose { fresh: false }));
        // a cell without surtitles ignores the slice
        turn(&mut m, Some(text()), T0);
        assert_eq!(turn(&mut m, Some(Input::Slice(6)), T0), None);
        assert_eq!(m.slice(), Some(6));
    }

    #[test]
    fn media_composes_again_only_for_a_cell_that_waited_for_it() {
        let mut m = machine();
        turn(&mut m, Some(text()), T0);
        m.composed(T0, 1.0, false, &["t: x.png not preloaded".to_string()], &None);
        assert_eq!(turn(&mut m, Some(Input::Media), T0), Some(Compose { fresh: false }));
        m.composed(T0, 1.0, false, &[], &None);
        assert_eq!(turn(&mut m, Some(Input::Media), T0), None, "nothing is missing any more");
    }

    #[test]
    fn a_resize_composes_the_cell_again_and_waits_for_a_cell() {
        let mut m = machine();
        assert_eq!(m.step(None, (1280, 720), T0, |_| true), None, "no cell: the size is not taken");
        assert_eq!(m.size(), SIZE);
        turn(&mut m, Some(text()), T0);
        assert_eq!(m.step(None, (1280, 720), T0, |_| true), Some(Compose { fresh: false }));
        assert_eq!(m.size(), (1280, 720));
        assert_eq!(m.step(None, (1280, 720), T0, |_| true), None);
    }

    #[test]
    fn the_scroll_steps_every_hold_and_a_new_cell_starts_it_again() {
        let mut m = machine();
        turn(&mut m, Some(scroll()), T0);
        let done = composed(&mut m, T0);
        assert_eq!(done.cell, "1 LAYERS SCROLL 0 12 MS");
        assert_eq!(m.wait(T0), POLL);
        assert_eq!(m.wait(T0 + SCROLL_HOLD - Duration::from_millis(50)), Duration::from_millis(50), "shorter than the poll when due sooner");
        assert_eq!(turn(&mut m, None, T0 + SCROLL_HOLD - Duration::from_millis(1)), None);
        let at = T0 + SCROLL_HOLD;
        assert_eq!(turn(&mut m, None, at), Some(Compose { fresh: false }));
        assert_eq!(m.scroll(), 1);
        assert_eq!(composed(&mut m, at).cell, "1 LAYERS SCROLL 1 12 MS");
        assert_eq!(turn(&mut m, None, at + SCROLL_HOLD), Some(Compose { fresh: false }));
        assert_eq!(m.scroll(), 2);
        composed(&mut m, at + SCROLL_HOLD);
        assert_eq!(turn(&mut m, Some(scroll()), at + SCROLL_HOLD), Some(Compose { fresh: true }));
        assert_eq!(m.scroll(), 0);
        // a cell that does not scroll stops the hold
        turn(&mut m, Some(text()), at);
        composed(&mut m, at);
        assert_eq!(m.wait(at + SCROLL_HOLD), POLL);
        assert_eq!(turn(&mut m, None, at + SCROLL_HOLD * 3), None);
    }

    #[test]
    fn the_space_node_hears_a_target_once() {
        let mut m = machine();
        turn(&mut m, Some(text()), T0);
        let a = Some(SpaceTarget { set: None, spot: Some("nave".into()) });
        let b = Some(SpaceTarget { set: None, spot: Some("choir".into()) });
        assert_eq!(m.composed(T0, 1.0, true, &[], &a).space, a);
        assert_eq!(m.composed(T0, 1.0, true, &[], &a).space, None, "unchanged");
        assert_eq!(m.composed(T0, 1.0, true, &[], &b).space, b);
        assert_eq!(m.composed(T0, 1.0, false, &[], &None).space, None, "no target: the node keeps what it has");
        assert_eq!(m.composed(T0, 1.0, false, &[], &b).space, b, "…and hears the same target again after a cell without one");
        assert_eq!(m.composed(T0, 2.0, true, &[], &None).cell, "1 LAYERS + SPACE 2 MS");
    }
}
