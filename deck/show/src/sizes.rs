//! The cue matrix's sizes as a viewer left them: each column's width, each row's height and the row heads' width, in
//! rem. They belong to the show's view, not to the show: the editor keeps them per show, the document never carries
//! them. The limits are decided here once: the smallest cell is a square, nothing grows past what a panel can hold.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// a cell as it comes: (width, height)
pub const CELL: (f32, f32) = (11.0, 3.6);
/// the row heads' width as it comes
pub const HEADS: f32 = 14.0;
/// the smallest side of a cell, both ways: the smallest cell is a square of a row's height as it comes
pub const MIN: f32 = CELL.1;
pub const MAX_WIDTH: f32 = 40.0;
pub const MAX_HEIGHT: f32 = 16.0;
pub const HEADS_MIN: f32 = 8.0;
pub const HEADS_MAX: f32 = 32.0;

/// What the viewer changed; a size back at its default is dropped.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sizes {
    /// by outlet or seat id
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub columns: BTreeMap<String, f32>,
    /// by row id
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rows: BTreeMap<String, f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heads: Option<f32>,
}

fn keep(map: &mut BTreeMap<String, f32>, id: &str, rem: f32, default: f32) {
    if (rem - default).abs() < 0.01 {
        map.remove(id);
    } else {
        map.insert(id.to_string(), rem);
    }
}

/// `rem` inside `[lo, hi]`, a size nobody can read (NaN) as `default`
fn limit(rem: f32, (lo, hi): (f32, f32), default: f32) -> f32 {
    if rem.is_finite() {
        rem.clamp(lo, hi)
    } else {
        default
    }
}

impl Sizes {
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty() && self.rows.is_empty() && self.heads.is_none()
    }

    pub fn column(&self, id: &str) -> f32 {
        self.columns.get(id).map_or(CELL.0, |w| limit(*w, (MIN, MAX_WIDTH), CELL.0))
    }

    pub fn row(&self, id: &str) -> f32 {
        self.rows.get(id).map_or(CELL.1, |h| limit(*h, (MIN, MAX_HEIGHT), CELL.1))
    }

    pub fn heads(&self) -> f32 {
        self.heads.map_or(HEADS, |w| limit(w, (HEADS_MIN, HEADS_MAX), HEADS))
    }

    /// a column dragged or fitted to `rem` (held to the limits); the width it came to
    pub fn set_column(&mut self, id: &str, rem: f32) -> f32 {
        let w = limit(rem, (MIN, MAX_WIDTH), CELL.0);
        keep(&mut self.columns, id, w, CELL.0);
        w
    }

    pub fn set_row(&mut self, id: &str, rem: f32) -> f32 {
        let h = limit(rem, (MIN, MAX_HEIGHT), CELL.1);
        keep(&mut self.rows, id, h, CELL.1);
        h
    }

    pub fn set_heads(&mut self, rem: f32) -> f32 {
        let w = limit(rem, (HEADS_MIN, HEADS_MAX), HEADS);
        self.heads = ((w - HEADS).abs() >= 0.01).then_some(w);
        w
    }

    /// A row renamed keeps its height.
    pub fn renamed_row(&mut self, from: &str, to: &str) {
        if let Some(h) = self.rows.remove(from) {
            self.rows.insert(to.to_string(), h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_hold_to_their_limits_the_smallest_cell_a_square_and_defaults_are_not_kept() {
        let mut s = Sizes::default();
        assert_eq!((s.column("left"), s.row("opening"), s.heads()), (CELL.0, CELL.1, HEADS));
        assert_eq!(s.set_column("left", 1.0), MIN, "no narrower than a square");
        assert_eq!(s.set_row("opening", 0.5), MIN);
        assert_eq!(s.column("left"), s.row("opening"), "the smallest cell is a square");
        assert_eq!(s.set_column("left", 400.0), MAX_WIDTH);
        assert_eq!(s.set_heads(2.0), HEADS_MIN);
        assert_eq!(s.set_column("right", f32::NAN), CELL.0);
        assert!(!s.columns.contains_key("right"), "a default is not kept");
        s.set_column("left", CELL.0);
        s.set_row("opening", CELL.1);
        s.set_heads(HEADS);
        assert!(s.is_empty());
        s.set_row("opening", 6.0);
        s.renamed_row("opening", "intro");
        assert_eq!(s.row("intro"), 6.0);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Sizes>(&json).unwrap(), s);
        // a hand-edited file outside the limits is read inside them
        let wild: Sizes = serde_json::from_str(r#"{ "columns": { "left": 0.1 } }"#).unwrap();
        assert_eq!(wild.column("left"), MIN);
    }
}
