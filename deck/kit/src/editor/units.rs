//! The unit-switch field (decisions/deck.md [design]): a value with a unit is shown in the unit chosen for its kind of
//! value, and the units it can be read in sit as small segments at the field's end. The catalogue and the conversions
//! are deck-core's `units`; the choice per kind of value is a view preference (`Prefs`), kept by the app. What a press
//! on a segment means stays with the caller.
use super::controls::{self, Rect};
use super::fold;
use crate::base::draw::DrawList;
use crate::base::tokens::Tokens;
use deck_core::units::{self, Refs, Unit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The unit chosen per kind of value ("crop" → px), a view preference: never stored on what the value belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Prefs(BTreeMap<String, String>);

impl Prefs {
    /// the unit `kind` is shown in: the chosen one if it can be offered with `refs`, else `fallback`'s dimension's
    pub fn unit(&self, kind: &str, fallback: Unit, refs: &Refs) -> Unit {
        units::pick(self.0.get(kind).and_then(|id| Unit::of_id(id)), fallback, refs)
    }

    pub fn set(&mut self, kind: &str, unit: Unit) {
        self.0.insert(kind.to_string(), unit.id().to_string());
    }
}

/// the switch's width for `offered` (none when there is nothing to switch)
pub fn switch_width(d: &DrawList, t: &Tokens, offered: &[Unit]) -> f32 {
    if offered.len() < 2 {
        return 0.0;
    }
    offered.iter().map(|u| d.sans_width(t.size.small, u.word()) + t.size.pad * 1.2).sum::<f32>() + 2.0 * (offered.len() - 1) as f32
}

/// The units `offered` as segments ending at `right`, `h` high from `y`, the one `on` pressed; each segment's place and
/// unit. Nothing when there is only one unit to read it in.
pub fn switch(d: &mut DrawList, t: &Tokens, (right, y, h): (f32, f32, f32), offered: &[Unit], on: Unit) -> Vec<(Rect, Unit)> {
    if offered.len() < 2 {
        return Vec::new();
    }
    let px = t.size.small;
    let mut x = right - switch_width(d, t, offered);
    let mut out = Vec::with_capacity(offered.len());
    for &u in offered {
        let w = d.sans_width(px, u.word()) + t.size.pad * 1.2;
        let lit = u == on;
        if lit {
            crate::editor::controls::active_tab(d, t, Rect { x, y, w, h }, t.size.radius);
        } else {
            d.outline(x, y, w, h, (t.size.radius, t.size.edge), t.color.line);
        }
        d.sans(x + t.size.pad * 0.6, y + (h - px * 1.25) * 0.5, px, u.word(), if lit { t.color.text } else { t.color.muted });
        out.push((Rect { x, y, w, h }, u));
        x += w + 2.0;
    }
    out
}

/// A value to drag, with its unit switch: the label, `v` (in `unit`) on its fill at `fraction`, the units `offered` at
/// its end. The field's place (the switch not in it) and the switch's segments.
#[allow(clippy::too_many_arguments)]
pub fn scrub(d: &mut DrawList, t: &Tokens, at: Rect, label: &str, v: f64, unit: Unit, offered: &[Unit], fraction: f32, hot: bool) -> (Rect, Vec<(Rect, Unit)>) {
    let sw = switch_width(d, t, offered);
    let gap = if sw > 0.0 { t.size.pad * 0.6 } else { 0.0 };
    let field = controls::scrub(d, t, Rect { w: at.w - sw - gap, ..at }, label, &units::show(v, unit), fraction, hot);
    let segs = switch(d, t, (at.x + at.w, field.y, field.h), offered, unit);
    (field, segs)
}

/// An exact value to type (fold's value field), with its unit switch: `v` (in `unit`), a note at its end. The field's
/// place and the switch's segments.
#[allow(clippy::too_many_arguments)]
pub fn exact(d: &mut DrawList, t: &Tokens, r: Rect, v: f64, unit: Unit, note: &str, offered: &[Unit], hot: bool) -> (Rect, Vec<(Rect, Unit)>) {
    let sw = switch_width(d, t, offered);
    let gap = if sw > 0.0 { t.size.pad * 0.6 } else { 0.0 };
    let field = fold::value(d, t, Rect { w: r.w - sw - gap, ..r }, &units::show(v, unit), note, hot);
    let segs = switch(d, t, (r.x + r.w, r.y, r.h), offered, unit);
    (field, segs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_is_kept_per_kind_and_falls_back_when_it_cannot_be_offered() {
        let mut p = Prefs::default();
        let picture = Refs { pixels: Some(1920.0), ..Refs::default() };
        assert_eq!(p.unit("crop", Unit::Percent, &picture), Unit::Percent, "nothing chosen: the fallback");
        p.set("crop", Unit::Pixels);
        assert_eq!(p.unit("crop", Unit::Percent, &picture), Unit::Pixels);
        assert_eq!(p.unit("crop", Unit::Percent, &Refs::default()), Unit::Percent, "no picture known: no pixels");
        assert_eq!(p.unit("fade", Unit::Seconds, &picture), Unit::Seconds, "another kind keeps its own");
        let text = toml::to_string(&std::collections::BTreeMap::from([("units", &p)])).unwrap();
        assert_eq!(text.trim(), "[units]
crop = \"px\"");
    }

    #[test]
    fn the_switch_shows_only_when_there_is_a_choice_and_ends_at_the_field() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let at = Rect { x: 0.0, y: 0.0, w: 300.0, h: t.size.row };
        let (field, segs) = scrub(&mut d, &t, at, "crop left", 192.0, Unit::Pixels, &[Unit::Pixels, Unit::Percent], 0.1, false);
        assert_eq!(segs.iter().map(|(_, u)| *u).collect::<Vec<_>>(), [Unit::Pixels, Unit::Percent]);
        let last = segs.last().unwrap().0;
        assert!((last.x + last.w - 300.0).abs() < 1e-3 && field.x + field.w < segs[0].0.x, "the switch at the end, beside the field");
        assert!(d.texts().contains(&"192 px"));
        let (field, segs) = scrub(&mut d, &t, at, "crop left", 10.0, Unit::Percent, &[Unit::Percent], 0.1, false);
        assert!(segs.is_empty() && field.x + field.w > 299.0, "one unit: no switch, the field takes the row");
    }
}
