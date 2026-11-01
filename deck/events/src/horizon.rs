//! Event horizons (#236, research/EVENTS.md §16 "Event horizons"): what a scope can reach from where it is, within a
//! bound, nearest first – the references a conductor, a deck or a reader preloads (or the engine states it pre-builds)
//! so a live show only waits for its trigger.
//!
//! A scope (a show's cue rows, a rig's mood variants, a set's objects, a chapter's triggers and choice branches) lists
//! its reachable items as [`Reach`]es – a reference, how far away it is ([`Dist`]: rows, seconds, choice depth) and what
//! loading it costs (bytes; 0 = already in the cache). [`walk`] orders them nearest first, keeps each reference once (at
//! its nearest), drops what lies past the [`Bound`] and fills the byte budget **as a prefix**: the first item that does
//! not fit ends the horizon, so a preloaded horizon never has a hole – everything nearer than the last ready item is
//! ready too. Platform-free, no clock, no I/O: the caller resolves the references.
use std::cmp::Ordering;
use std::collections::BTreeSet;

/// How far an item is from the current position. Nearest first = by seconds, then rows, then choice depth.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dist {
    /// rows ahead (a show's cue rows, a chapter's steps, a ladder's rungs)
    pub rows: u32,
    /// seconds ahead (on the timeline; 0 for a trigger that can come at any time)
    pub secs: f64,
    /// choice branches taken to get there (0 = on the current path)
    pub depth: u32,
}

impl Dist {
    pub fn rows(rows: u32) -> Dist {
        Dist { rows, ..Dist::default() }
    }

    fn order(&self, other: &Dist) -> Ordering {
        self.secs.total_cmp(&other.secs).then(self.rows.cmp(&other.rows)).then(self.depth.cmp(&other.depth))
    }
}

/// A reachable item: a reference (an id, an immutable path, a mood …), its distance, its loading cost.
#[derive(Debug, Clone, PartialEq)]
pub struct Reach<R> {
    pub item: R,
    pub dist: Dist,
    /// bytes to load (0 = cached, or free to build)
    pub bytes: u64,
}

impl<R> Reach<R> {
    pub fn new(item: R, dist: Dist, bytes: u64) -> Reach<R> {
        Reach { item, dist, bytes }
    }
}

/// The defaults (owner 2026-11-01: provisional, tuned after real-show tests): a show's rolling horizon looks this many
/// cue rows and seconds ahead (whichever ends first), a chapter this many choices deep, a deck loads at most this many
/// bytes ahead. Past the budget a deck loads on the trigger (late – the show never stops), it does not refuse the cue.
pub const SHOW_ROWS: u32 = 8;
pub const SHOW_SECS: f64 = 60.0;
pub const CHOICE_DEPTH: u32 = 2;
pub const DECK_BYTES: u64 = 256 * 1024 * 1024;

/// The horizon's limits; `None` = unbounded in that dimension.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bound {
    /// at most this many rows ahead
    pub rows: Option<u32>,
    /// at most this many seconds ahead
    pub secs: Option<f64>,
    /// at most this many choice branches deep
    pub depth: Option<u32>,
    /// at most this many bytes to load, in total
    pub bytes: Option<u64>,
    /// at most this many items
    pub items: Option<usize>,
}

impl Bound {
    /// everything (a show's whole horizon before doors)
    pub fn all() -> Bound {
        Bound::default()
    }

    /// a show's rolling horizon during it (before doors: `all`, the whole show)
    pub fn show() -> Bound {
        Bound { rows: Some(SHOW_ROWS), secs: Some(SHOW_SECS), bytes: Some(DECK_BYTES), ..Bound::default() }
    }

    /// a chapter's triggers and choice branches
    pub fn chapter() -> Bound {
        Bound { depth: Some(CHOICE_DEPTH), bytes: Some(DECK_BYTES), ..Bound::default() }
    }

    fn within(&self, d: &Dist) -> bool {
        self.rows.map_or(true, |r| d.rows <= r)
            && self.secs.map_or(true, |s| d.secs <= s)
            && self.depth.map_or(true, |c| d.depth <= c)
    }
}

/// Why the horizon ended before the scope's last reachable item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    /// the next item would have gone over the byte budget
    Budget,
    /// the item count was reached
    Items,
}

/// The walk's result: the references to preload, nearest first.
#[derive(Debug, Clone, PartialEq)]
pub struct Horizon<R> {
    pub items: Vec<Reach<R>>,
    /// the bytes `items` cost
    pub bytes: u64,
    /// within rows / secs / depth but left out by the budget or the count (`None` = nothing left out)
    pub cut: Option<Cut>,
}

impl<R> Horizon<R> {
    pub fn refs(&self) -> impl Iterator<Item = &R> + '_ {
        self.items.iter().map(|r| &r.item)
    }
}

/// Walk a scope's reachable items: nearest first (ties keep the scope's order), each reference once at its nearest,
/// within the bound, the budget filled as a prefix.
pub fn walk<R: Ord + Clone>(reach: impl IntoIterator<Item = Reach<R>>, bound: &Bound) -> Horizon<R> {
    let mut all: Vec<Reach<R>> = reach.into_iter().filter(|r| bound.within(&r.dist)).collect();
    // stable: equal distances keep the scope's order
    all.sort_by(|a, b| a.dist.order(&b.dist));
    let mut seen = BTreeSet::new();
    let mut out = Horizon { items: Vec::new(), bytes: 0, cut: None };
    for r in all {
        if !seen.insert(r.item.clone()) {
            continue;
        }
        if bound.items.is_some_and(|n| out.items.len() >= n) {
            out.cut = Some(Cut::Items);
            break;
        }
        let bytes = out.bytes.saturating_add(r.bytes);
        if bound.bytes.is_some_and(|b| bytes > b) {
            out.cut = Some(Cut::Budget);
            break;
        }
        out.bytes = bytes;
        out.items.push(r);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(item: &str, rows: u32, bytes: u64) -> Reach<String> {
        Reach::new(item.to_string(), Dist::rows(rows), bytes)
    }

    fn names(h: &Horizon<String>) -> Vec<&str> {
        h.refs().map(|s| s.as_str()).collect()
    }

    #[test]
    fn nearest_first_and_ties_keep_the_scopes_order() {
        let h = walk([r("c", 2, 1), r("a", 0, 1), r("b2", 1, 1), r("b1", 1, 1)], &Bound::all());
        assert_eq!(names(&h), ["a", "b2", "b1", "c"]);
        assert_eq!((h.bytes, h.cut), (4, None));
    }

    #[test]
    fn seconds_order_before_rows_and_rows_before_depth() {
        let at = |item: &str, rows, secs, depth| Reach::new(item.to_string(), Dist { rows, secs, depth }, 0);
        let h = walk([at("late", 0, 5.0, 0), at("branch", 1, 0.0, 1), at("row", 1, 0.0, 0), at("now", 0, 0.0, 0)], &Bound::all());
        assert_eq!(names(&h), ["now", "row", "branch", "late"]);
    }

    #[test]
    fn rows_seconds_and_depth_bound_the_walk() {
        let at = |item: &str, rows, secs, depth| Reach::new(item.to_string(), Dist { rows, secs, depth }, 0);
        let scope = [at("a", 0, 0.0, 0), at("far_row", 9, 0.0, 0), at("far_time", 0, 60.0, 0), at("deep", 0, 0.0, 3)];
        let h = walk(scope.clone(), &Bound { rows: Some(2), secs: Some(30.0), depth: Some(1), ..Bound::all() });
        assert_eq!(names(&h), ["a"]);
        assert_eq!(h.cut, None, "out of rows / secs / depth is not a cut – it is outside the horizon");
        assert_eq!(walk(scope, &Bound::all()).items.len(), 4);
    }

    #[test]
    fn a_reference_counts_once_at_its_nearest() {
        let h = walk([r("set:cathedral", 3, 100), r("char:catpope", 1, 10), r("set:cathedral", 0, 100)], &Bound::all());
        assert_eq!(names(&h), ["set:cathedral", "char:catpope"]);
        assert_eq!(h.items[0].dist.rows, 0);
        assert_eq!(h.bytes, 110, "its bytes once");
    }

    #[test]
    fn the_budget_is_filled_as_a_prefix() {
        // b does not fit: c (small, but farther) is left out too – no holes in a preloaded horizon
        let h = walk([r("a", 0, 60), r("b", 1, 50), r("c", 2, 1)], &Bound { bytes: Some(100), ..Bound::all() });
        assert_eq!(names(&h), ["a"]);
        assert_eq!((h.bytes, h.cut), (60, Some(Cut::Budget)));
        // cached items cost nothing and always fit
        let h = walk([r("a", 0, 100), r("cached", 1, 0)], &Bound { bytes: Some(100), ..Bound::all() });
        assert_eq!((names(&h), h.cut), (vec!["a", "cached"], None));
    }

    #[test]
    fn the_item_count_cuts() {
        let h = walk([r("a", 0, 1), r("b", 1, 1), r("c", 2, 1)], &Bound { items: Some(2), ..Bound::all() });
        assert_eq!((names(&h), h.cut), (vec!["a", "b"], Some(Cut::Items)));
        let h = walk([r("a", 0, 1), r("b", 1, 1)], &Bound { items: Some(2), ..Bound::all() });
        assert_eq!(h.cut, None, "exactly full is not a cut");
    }

    #[test]
    fn the_defaults_bound_a_show_and_a_chapter() {
        let at = |item: &str, rows, secs, depth| Reach::new(item.to_string(), Dist { rows, secs, depth }, 0);
        let show = [at("next", 1, 4.0, 0), at("row8", 8, 50.0, 0), at("row9", 9, 55.0, 0), at("late", 2, 61.0, 0)];
        assert_eq!(names(&walk(show, &Bound::show())), ["next", "row8"], "8 rows or 60 s, whichever ends first");
        let chapter = [at("choice", 1, 0.0, 1), at("two", 2, 0.0, 2), at("three", 3, 0.0, 3)];
        assert_eq!(names(&walk(chapter, &Bound::chapter())), ["choice", "two"]);
        let big = [r("set", 0, DECK_BYTES), r("more", 1, 1)];
        assert_eq!(walk(big, &Bound::show()).cut, Some(Cut::Budget), "past the deck's budget: loaded on the trigger");
    }

    #[test]
    fn an_empty_scope_is_an_empty_horizon() {
        let h = walk(Vec::<Reach<String>>::new(), &Bound { bytes: Some(0), ..Bound::all() });
        assert_eq!((h.items.len(), h.bytes, h.cut), (0, 0, None));
    }
}
