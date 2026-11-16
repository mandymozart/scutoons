//! The one condition control (#553, SHOW.md §15 2b; owner 2026-11-14: row and cell conditions "should be very similar and
//! perhaps share a UI … so we have recognisable elements"): a label and the filter's words on one tree row, then its facts
//! as deck-kit chips – the look of the role filter (#478). Drawn for a role's admission (roles.rs), a row address's filter
//! (author.rs) and a cell's column filter (rows.rs); #551's question form calls it the same way. The model – the facts
//! offered, which are on, the words, the mode (any one fact / every fact) – is deck-ui `filter::Condition`, built by
//! deck-show (`roles::condition`, `rows::address_condition`, `rows::cell_condition`); a chip pressed is `Condition::fact`,
//! handed to the caller's deck-show edit. A scene slot's filter (deck-space `scene_slots::FillQuestion`) is drawn by the
//! same control (`chips`). This file only lays it out (RULES A1, A2).
use super::roles::line;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_ui::filter::Condition;

/// Draws `c` under `label` at `(x, y, w)`: the chips' places (the place of chip `i` presses `c.fact(i)`) and the height
/// used, a gap below included.
pub fn draw(d: &mut DrawList, t: &Tokens, at: (f32, f32, f32), label: &str, c: &Condition) -> (Vec<Rect>, f32) {
    let (names, on) = c.chips();
    chips(d, t, at, label, &c.words, &names, &on)
}

/// the control for any condition model: its words, then its chips
pub fn chips(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), label: &str, words: &str, names: &[&str], on: &[bool]) -> (Vec<Rect>, f32) {
    line(d, t, (x, y, w), label, words);
    let (places, used) = controls::chips(d, t, (x, y + t.size.tree_row, w), names, on);
    (places, t.size.tree_row + used + t.size.pad * 0.5)
}
