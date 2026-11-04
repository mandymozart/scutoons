//! gamepad.rs (#228, #369): a gamepad on the deck's two sticks and its selection – the rules every deck shares, once: the
//! browser's Gamepad API (space-web's `DeckControls::pad`) and the native stage's gilrs (deck-native's pad adapter) only
//! hand in the pad's state each frame (the Gamepad API's standard mapping) and do what the events say, the same calls the
//! touch sticks make (`Controls::stick_down` / `stick_move` / `stick_up`).
//!
//! - A stick goes **down** once it is pushed past `PRESS`, follows its deflection while it is out, and goes **up** once it
//!   is back inside `RELEASE` – a little lower, so a stick resting near the edge does not chatter.
//! - The deflection is the stick's (−1..1 per axis) with **y up** – the standard mapping reports y down.
//! - **A** (south, button 0) cycles the selection forward on its press edge, **Y** (north, button 3) back.
//! - A pad that goes away: the host hands in an empty state – every stick that was down goes up.
//!
//! Never allocates.

/// a stick pushed this far out (its reach, 0..1) goes down
pub const PRESS: f32 = 0.3;
/// a stick that is down goes up once it is back this close to rest
pub const RELEASE: f32 = 0.15;
/// each stick's (x, y) axes in the standard mapping: the left stick 0 / 1, the right 2 / 3
pub const AXES: [[usize; 2]; 2] = [[0, 1], [2, 3]];
/// the button that cycles the selection forward (south: A / cross)
pub const NEXT: usize = 0;
/// the button that cycles it back (north: Y / triangle)
pub const BACK: usize = 3;

/// What one poll asks of the host, in order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadEvent {
    /// stick `side` went down: a thumb landed (`Controls::stick_down` with its values now)
    Down(usize),
    /// stick `side`'s deflection (−1..1, y up): `Controls::stick_move`
    Move(usize, f32, f32),
    /// stick `side` went up
    Up(usize),
    /// the selection moved on (`back`: to the previous one) – `Controls::pad` has already moved it
    Cycle { back: bool },
}

impl PadEvent {
    /// as four numbers – `[kind, side, dx, dy]`, kind 0 down · 1 move · 2 up · 3 cycle (side 1 = back) – for the wasm API
    pub fn code(&self) -> [f32; 4] {
        match *self {
            PadEvent::Down(s) => [0.0, s as f32, 0.0, 0.0],
            PadEvent::Move(s, x, y) => [1.0, s as f32, x, y],
            PadEvent::Up(s) => [2.0, s as f32, 0.0, 0.0],
            PadEvent::Cycle { back } => [3.0, if back { 1.0 } else { 0.0 }, 0.0, 0.0],
        }
    }
}

/// One poll's events (at most: each stick down and moved, two buttons) – no allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PadEvents {
    list: [Option<PadEvent>; 6],
    len: usize,
}

impl PadEvents {
    fn push(&mut self, e: PadEvent) {
        if self.len < self.list.len() {
            self.list[self.len] = Some(e);
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = PadEvent> + '_ {
        self.list[..self.len].iter().flatten().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// The pad's state between polls: which sticks are down, which buttons were held.
#[derive(Debug, Clone, Default)]
pub struct Pad {
    down: [bool; 2],
    held: [bool; 2],
}

impl Pad {
    /// One poll: the pad's axes and buttons in the standard mapping (missing or non-finite ones read as rest / up; an empty
    /// state = no pad) → what the host does.
    pub fn poll(&mut self, axes: &[f32], buttons: &[bool]) -> PadEvents {
        let axis = |i: usize| axes.get(i).copied().filter(|v| v.is_finite()).unwrap_or(0.0).clamp(-1.0, 1.0);
        let mut out = PadEvents::default();
        for (side, [ax, ay]) in AXES.into_iter().enumerate() {
            let (x, y) = (axis(ax), -axis(ay));
            let reach = x.hypot(y);
            if !self.down[side] {
                if reach < PRESS {
                    continue;
                }
                self.down[side] = true;
                out.push(PadEvent::Down(side));
            } else if reach <= RELEASE {
                self.down[side] = false;
                out.push(PadEvent::Up(side));
                continue;
            }
            out.push(PadEvent::Move(side, x, y));
        }
        for (i, (button, back)) in [(NEXT, false), (BACK, true)].into_iter().enumerate() {
            let pressed = buttons.get(button).copied().unwrap_or(false);
            if pressed && !self.held[i] {
                out.push(PadEvent::Cycle { back });
            }
            self.held[i] = pressed;
        }
        out
    }

    pub fn is_down(&self, side: usize) -> bool {
        self.down[side.min(1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PadEvent::*;

    fn poll(p: &mut Pad, axes: &[f32], buttons: &[bool]) -> Vec<PadEvent> {
        p.poll(axes, buttons).iter().collect()
    }

    /// owner 2026-10-31 (#228): a stick pushed past the press point goes down and follows (y up); between the points it still
    /// follows; inside the release point it goes up; a stick that never reaches the press point never goes down
    #[test]
    fn a_stick_goes_down_past_the_press_point_and_up_inside_the_release_point() {
        let mut p = Pad::default();
        assert!(poll(&mut p, &[0.1, 0.0], &[]).is_empty(), "a resting stick does nothing");
        assert_eq!(poll(&mut p, &[0.6, -0.8], &[]), [Down(0), Move(0, 0.6, 0.8)], "the axis y is down-positive");
        assert_eq!(poll(&mut p, &[0.2, 0.0], &[]), [Move(0, 0.2, -0.0)]);
        assert_eq!(poll(&mut p, &[0.1, 0.0], &[]), [Up(0)]);
        assert!(poll(&mut p, &[0.1, 0.0], &[]).is_empty());
        for x in [0.05, -0.12, 0.2, 0.25] {
            assert!(poll(&mut p, &[x, 0.0], &[]).is_empty(), "{x}");
        }
        assert_eq!(poll(&mut p, &[0.0, 0.0, 0.9, 0.0], &[]), [Down(1), Move(1, 0.9, -0.0)], "the right stick: axes 2 and 3");
        assert_eq!(poll(&mut p, &[], &[]), [Up(1)], "no pad: the sticks that were down go up");
        assert!(poll(&mut p, &[f32::NAN, f32::INFINITY], &[]).is_empty(), "a broken axis is rest");
    }

    #[test]
    fn a_and_y_cycle_on_their_press_edge() {
        let mut p = Pad::default();
        assert_eq!(poll(&mut p, &[], &[true]), [Cycle { back: false }]);
        assert!(poll(&mut p, &[], &[true]).is_empty(), "held: nothing");
        assert!(poll(&mut p, &[], &[false]).is_empty());
        assert_eq!(poll(&mut p, &[], &[false, false, false, true]), [Cycle { back: true }]);
        assert_eq!(Cycle { back: true }.code(), [3.0, 1.0, 0.0, 0.0]);
        assert_eq!(Move(1, 0.5, -0.25).code(), [1.0, 1.0, 0.5, -0.25]);
    }
}
