//! A cell's values against its piece's knobs (SHOW.md §5): the cell inspector's knobs as scrub numbers (a value the
//! cell sets, else the piece's own), one value set or removed and checked against the knob and its range, "take current"
//! (what plays on the column now, the piece's own for the rest), and the warning where a `values` cell meets another piece
//! on the rows' path. The knobs are the core's (deck-space `piece::knobs`, the editor's loader hands them in); a value's key
//! is read by the one parser (`piece::slot`: a bare name is its node's first knob).
use crate::conductor::Playing;
use crate::document::{Cell, Show, Values};
use crate::Subtree;
use deck_space::piece::slot;
pub use deck_space::rig::bundle::Knob;

/// the cell inspector's label over the knobs, its button, and what it says while the knobs are on their way
pub const KNOBS: &str = "values";
pub const TAKE: &str = "take current";
pub const LOADING: &str = "the piece's knobs are on their way";

/// One knob of the cell's piece as the inspector shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct KnobView {
    /// the slot (`dim.gain`): what a turn writes
    pub path: String,
    pub label: String,
    pub min: f32,
    pub max: f32,
    pub log: bool,
    /// the cell's value for it, else the piece's own
    pub value: f32,
    /// the cell sets it
    pub set: bool,
}

/// the knob among `knobs` that the cell's value `key` sets
fn knob_of<'a>(knobs: &'a [Knob], key: &str) -> Result<&'a Knob, String> {
    let s = slot(knobs, key).map_err(|e| format!("the value '{key}': {e}"))?;
    s.and_then(|s| knobs.iter().find(|k| k.path == s)).ok_or_else(|| format!("the value '{key}' turns no knob of the piece"))
}

/// a number for a value as the document keeps it (f32's shortest words: 0.4, not 0.4000000059604645)
fn json(v: f32) -> serde_json::Value {
    v.to_string().parse::<serde_json::Number>().map_or(serde_json::Value::Null, serde_json::Value::Number)
}

/// The knobs of the piece with the cell's `values` over the piece's own, in the piece's order; `locale` picks the labels.
pub fn knob_views(values: &Values, knobs: &[Knob], locale: &str) -> Vec<KnobView> {
    knobs
        .iter()
        .map(|k| {
            let set = values.iter().find(|(key, _)| knob_of(knobs, key).is_ok_and(|x| x.path == k.path)).and_then(|(_, v)| v.as_f64());
            let label = deck_space::rig::bundle::pick_label(&k.label, locale).map_or_else(|| k.path.clone(), str::to_string);
            KnobView { path: k.path.clone(), label, min: k.min, max: k.max, log: k.log, value: set.map_or(k.value, |v| v as f32), set: set.is_some() }
        })
        .collect()
}

/// Whether `values` fit the piece: every number turns one of its knobs and stays in its range (`mood`, a rig's word, is
/// not a knob). Err: the first that does not, in words.
pub fn check(values: &Values, knobs: &[Knob]) -> Result<(), String> {
    for (key, v) in values {
        if key == "mood" && v.is_string() {
            continue;
        }
        let k = knob_of(knobs, key)?;
        let n = v.as_f64().ok_or_else(|| format!("the value '{key}' is no number"))? as f32;
        if !(k.min..=k.max).contains(&n) {
            return Err(format!("{key} goes from {} to {}, not {}", json(k.min), json(k.max), json(n)));
        }
    }
    Ok(())
}

/// Knob `path` set to `to` in `values` (none: removed), under the key the cell already names it by, else its path. The
/// words of what changed.
pub fn set(values: &mut Values, knobs: &[Knob], path: &str, to: Option<f32>) -> Result<String, String> {
    let k = knob_of(knobs, path)?;
    let key = values.keys().find(|key| knob_of(knobs, key).is_ok_and(|x| x.path == k.path)).cloned();
    match (to, key) {
        (Some(v), _) if !(k.min..=k.max).contains(&v) => Err(format!("{} goes from {} to {}, not {}", k.path, json(k.min), json(k.max), json(v))),
        (Some(v), key) => {
            values.insert(key.unwrap_or_else(|| k.path.clone()), json(v));
            Ok(format!("{} {}", k.path, json(v)))
        }
        (None, Some(key)) => {
            values.remove(&key);
            Ok(format!("{} is the piece's own again", k.path))
        }
        (None, None) => Err(format!("the cell sets no {}", k.path)),
    }
}

/// "take current": every knob of `piece` where it stands on `column` now – the value the conductor sent there, else the
/// piece's own. Err: the column plays another piece, or none.
pub fn current(sub: &Subtree, column: &str, piece: &str, knobs: &[Knob]) -> Result<Vec<(String, f32)>, String> {
    let sent = match sub.conductor().on_stage(column).map(|s| &s.playing) {
        Some(Playing::Piece { piece: there, values }) if there == piece => values,
        Some(Playing::Piece { piece: there, .. }) => return Err(format!("{column} plays {there}, not {piece} – nothing to take")),
        _ => return Err(format!("nothing plays on {column} – fire a row that plays {piece} first")),
    };
    Ok(knob_views(sent, knobs, "en").into_iter().map(|k| (k.path, k.value)).collect())
}

/// Every knob in `positions` written into `values` (the keys it names them by kept). The count.
pub fn take(values: &mut Values, knobs: &[Knob], positions: &[(String, f32)]) -> Result<usize, String> {
    for (path, v) in positions {
        set(values, knobs, path, Some(*v))?;
    }
    Ok(positions.len())
}

/// Build's knob positions (#568: the rig open there, its own slots) as the cell's piece names them: `prefix` before each
/// (a scene's member: `<holder>_`, "" when the piece's slots are the rig's), only what turns a knob of the piece – the
/// rest is counted, not taken. For [`take`] (through the rows' `Edit::Take`, which checks the ranges).
pub fn from_build(positions: &[(String, f32)], prefix: &str, knobs: &[Knob]) -> (Vec<(String, f32)>, usize) {
    let (mut out, mut left) = (Vec::new(), 0);
    for (path, v) in positions {
        let slot = format!("{prefix}{path}");
        if knobs.iter().any(|k| k.path == slot) {
            out.push((slot, *v));
        } else {
            left += 1;
        }
    }
    (out, left)
}

/// A `values` cell that meets another piece (or none) on the rows' path: the row above that last changed its column plays
/// something else, so at the show it is skipped and logged (the conductor's `ValuesSkipped`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub row: String,
    pub column: String,
    pub text: String,
}

/// The warnings of `show`, rows top down (the path a show runs in order; a row fired out of order is the log's to say).
pub fn warnings(show: &Show) -> Vec<Warning> {
    let mut out = vec![];
    for (i, r) in show.rows.iter().enumerate() {
        for (column, cell) in &r.cells {
            let Cell::Values { piece, .. } = cell else { continue };
            let before = show.rows[..i].iter().enumerate().rev().find(|(_, x)| !matches!(x.cells.get(column), None | Some(Cell::Keep)));
            let there = before.map(|(j, x)| (j, x, x.cells.get(column).and_then(|c| match c {
                Cell::Play { piece: Some(p), .. } | Cell::Values { piece: p, .. } => Some(p.as_str()),
                _ => None,
            })));
            let found = match there {
                Some((_, _, Some(p))) if p == piece => continue,
                Some((j, x, Some(p))) => format!("{p} plays there (row {} {})", j + 1, x.id),
                Some((j, x, None)) => format!("nothing plays there (row {} {}: {})", j + 1, x.id, crate::view::cell_text(&x.cells[column])),
                None => "no row above plays a piece there".to_string(),
            };
            out.push(Warning { row: r.id.clone(), column: column.clone(), text: format!("row {} {}, {column}: values for {piece}, but {found} – skipped at the show", i + 1, r.id) });
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::document::parse;
    use crate::rows::{edit, edit_with, Edit};

    /// the nave of the fixtures as the core reads it: a tone (`light`) and a noise (`dim`) into a mixer
    pub(crate) fn nave() -> Vec<Knob> {
        let k = |path: &str, label: &str, value: f32| Knob { path: path.into(), label: [("en".to_string(), label.to_string())].into(), min: 0.0, max: 1.0, value, log: false };
        vec![k("light.gain", "Light", 0.3), k("dim.gain", "Dim", 0.6)]
    }

    fn value(row: &str, column: &str, path: &str, to: Option<f32>) -> Edit {
        Edit::Value { row: row.into(), column: column.into(), path: path.into(), to }
    }

    /// the knobs come from the core's piece loader: a scene's TOML in, its exposed knobs out (bare names by the one parser)
    #[test]
    fn the_core_loads_a_pieces_knobs_and_reads_a_bare_name() {
        let knobs = deck_space::piece::knobs("scene:nave", |_| Some(deck_space::piece::Found::Scene(NAVE.into()))).unwrap();
        assert!(!knobs.is_empty(), "the scene exposes knobs");
        let first = &knobs[0];
        let node = first.path.split('.').next().unwrap();
        assert_eq!(slot(&knobs, node).unwrap().as_deref(), Some(first.path.as_str()), "a bare name is its node's first knob");
        assert!(deck_space::piece::knobs("scene:nave", |_| None).unwrap_err().contains("not on this deck"));
    }

    /// one value set (the key the cell names it by kept), one added under its path, one removed, each checked
    #[test]
    fn a_value_is_set_and_removed_and_checked_against_the_knobs() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let knobs = nave();
        let set = edit_with(&show, &value("dialog", "left", "dim.gain", Some(0.25)), Some(&knobs)).unwrap();
        assert_eq!(set.said, "row 2 dialog, left: dim.gain 0.25");
        let cell = set.show.row("dialog").unwrap().cells.get("left").unwrap().clone();
        assert_eq!(cell, Cell::Values { piece: "scene:nave".into(), values: serde_json::from_str(r#"{ "dim": 0.25 }"#).unwrap() }, "the bare name stays");
        let added = edit_with(&set.show, &value("dialog", "left", "light.gain", Some(0.7)), Some(&knobs)).unwrap();
        assert_eq!(crate::view::cell_text(added.show.row("dialog").unwrap().cells.get("left").unwrap()), "values · dim 0.25 · light.gain 0.7");
        let removed = edit_with(&added.show, &value("dialog", "left", "dim.gain", None), Some(&knobs)).unwrap();
        assert_eq!(removed.said, "row 2 dialog, left: dim.gain is the piece's own again");
        // out of range, a knob the piece lacks, no knobs known, a cell without a piece
        assert_eq!(edit_with(&show, &value("dialog", "left", "dim.gain", Some(1.5)), Some(&knobs)).unwrap_err(), "not changed: dim.gain goes from 0 to 1, not 1.5");
        assert_eq!(edit_with(&show, &value("dialog", "left", "wind.gain", Some(0.5)), Some(&knobs)).unwrap_err(), "not changed: the value 'wind.gain' turns no knob of the piece");
        assert!(edit(&show, &value("dialog", "left", "dim.gain", Some(0.5))).unwrap_err().contains("knobs are not known"));
        assert!(edit_with(&show, &value("finale", "lamps", "dim.gain", Some(0.5)), Some(&knobs)).unwrap_err().contains("names no piece"));
    }

    #[test]
    fn the_inspectors_knobs_say_the_cells_value_or_the_pieces_own() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let Some(Cell::Values { values, .. }) = show.row("dialog").unwrap().cells.get("left") else { panic!() };
        let v = knob_views(values, &nave(), "en");
        assert_eq!(v.iter().map(|k| (k.label.as_str(), k.value, k.set)).collect::<Vec<_>>(), [("Light", 0.3, false), ("Dim", 0.5, true)]);
        assert!(check(values, &nave()).is_ok());
    }

    /// "take current": the conductor's values on the column, the piece's own for the rest
    #[test]
    fn take_current_copies_what_plays_into_the_cell() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        let mut sub = Subtree::new(show.clone(), crate::ClockChoice::Parent);
        let knobs = nave();
        assert!(current(&sub, "left", "scene:nave", &knobs).unwrap_err().starts_with("nothing plays on left"));
        sub.fire("opening", 1000);
        let now = current(&sub, "left", "scene:nave", &knobs).unwrap();
        assert_eq!(now, [("light.gain".to_string(), 0.2), ("dim.gain".to_string(), 0.6)], "opening sets light 0.2; dim is the piece's own");
        assert!(current(&sub, "left", "scene:vigil", &knobs).unwrap_err().contains("plays scene:nave, not scene:vigil"));
        let took = edit_with(&show, &Edit::Take { row: "dialog".into(), column: "left".into(), positions: now }, Some(&knobs)).unwrap();
        assert_eq!(took.said, "row 2 dialog, left: took 2 knobs – values · dim 0.6 · light.gain 0.2");
    }

    /// #568: Build's knobs into the cell – the piece's slots taken (a member's prefix before them), the rest counted, the
    /// ranges the rows' edit's
    #[test]
    fn builds_knobs_go_into_the_cell_as_the_piece_names_them() {
        let show = parse(crate::far::tests::SHOW).unwrap();
        let knobs = deck_space::holders::read("character:keys", crate::far::tests::find).unwrap().knobs;
        let build = [("tone_osc.gain".to_string(), 0.5), ("tone_osc.freq".to_string(), 220.0), ("far_in.x".to_string(), 1.0)];
        let (taken, left) = from_build(&build, "", &knobs);
        assert_eq!((taken.len(), left), (2, 1), "{knobs:?}");
        let done = edit_with(&show, &Edit::Take { row: "together".into(), column: "left".into(), positions: taken }, Some(&knobs)).unwrap();
        assert!(done.said.starts_with("row 2 together, left: took 2 knobs"), "{}", done.said);
        assert_eq!(from_build(&build, "keys_", &knobs).0, vec![], "a member's prefix the piece does not carry");
        let too_high = [("tone_osc.freq".to_string(), 5000.0)];
        assert!(edit_with(&show, &Edit::Take { row: "together".into(), column: "left".into(), positions: from_build(&too_high, "", &knobs).0 }, Some(&knobs)).unwrap_err().contains("goes from"));
    }

    /// SHOW.md §5: a `values` cell that meets another piece on the path is warned about, in the conductor's words
    #[test]
    fn a_values_cell_meeting_another_piece_is_warned_about() {
        let show = parse(include_str!("../fixtures/top-toy.show.json")).unwrap();
        assert!(warnings(&show).is_empty(), "dialog's values follow opening's nave: {:?}", warnings(&show));
        let swapped = edit(&show, &Edit::Piece { row: "dialog".into(), column: "left".into(), piece: "scene:vigil".into() }).unwrap().show;
        let w = warnings(&swapped);
        assert_eq!(w, [Warning { row: "dialog".into(), column: "left".into(), text: "row 2 dialog, left: values for scene:vigil, but scene:nave plays there (row 1 opening) – skipped at the show".into() }]);
        let listed = crate::view::problems(&swapped, None);
        assert!(listed.iter().any(|p| p.text == w[0].text && !p.error && p.points == crate::view::Points::Cell(1, 0)), "the problems list warns: {listed:?}");
        let mut first = show.clone();
        first.rows.remove(0);
        assert_eq!(warnings(&first)[0].text, "row 1 dialog, left: values for scene:nave, but no row above plays a piece there – skipped at the show");
    }

    /// a scene that exposes two knobs (deck-space piece.rs's stand-in for the nave)
    const NAVE: &str = r#"
[stage]
name = "nave"
transition_secs = 0.5

[being]
expose = ["light.gain", "dim.gain"]

[[node]]
id = "light"
kind = "osc"
freq = 220.0
gain = 0.8

[[node]]
id = "dim"
kind = "noise"
gain = 0.3

[[node]]
id = "mix"
kind = "mixer"

[[node]]
id = "out"
kind = "audio_out"

[[link]]
from = "light"
to = "mix.in1"

[[link]]
from = "dim"
to = "mix.in2"

[[link]]
from = "mix"
to = "out.in"
"#;
}
