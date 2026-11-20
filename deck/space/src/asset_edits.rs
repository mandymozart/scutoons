//! An asset's edit list and a derivative's time map as the API carries them (`GET /api/assets/{id}`: the asset's
//! `edits`, each rendition's `time`): read into deck-core's `Edits` (which cleans every number) and written back. The
//! server stores what the core wrote; anything it cannot read is no edit.
//!
//! `edits`: `{ "start", "end", "crop": { "left", "top", "right", "bottom" }, "fade_in", "fade_out", "speed", "gain_db" }`,
//! each key left out when it is none. `time`: `{ "offset", "fps" }`.
use deck_core::asset_edit::{Crop, Edits};
use deck_core::trim::Trim;
use serde_json::{json, Map, Value};

fn num(v: &Value, key: &str, or: f64) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(or)
}

/// the asset's `edits` (absent, null or not an object: none)
pub fn edits(v: &Value) -> Edits {
    if !v.is_object() {
        return Edits::NONE;
    }
    let c = &v["crop"];
    let part = |k: &str| num(c, k, 0.0) as f32;
    Edits::new(
        Trim::new(num(v, "start", 0.0), num(v, "end", 0.0)),
        Crop::new(part("left"), part("top"), part("right"), part("bottom")),
        num(v, "fade_in", 0.0),
        num(v, "fade_out", 0.0),
        num(v, "speed", 1.0),
        num(v, "gain_db", 0.0),
    )
}

/// the `edits` to store: only what differs from none (no edits: null)
pub fn to_json(e: &Edits) -> Value {
    if e.none() {
        return Value::Null;
    }
    let round = |x: f64| (x * 1e4).round() / 1e4;
    let mut m = Map::new();
    let mut put = |k: &str, x: f64, none: f64| {
        if (x - none).abs() > 1e-9 {
            m.insert(k.into(), json!(round(x)));
        }
    };
    put("start", e.trim.start, 0.0);
    put("end", e.trim.end, 0.0);
    put("fade_in", e.fade_in, 0.0);
    put("fade_out", e.fade_out, 0.0);
    put("speed", e.speed, 1.0);
    put("gain_db", e.gain_db, 0.0);
    if !e.crop.none() {
        let c = e.crop;
        let r = |x: f32| round(f64::from(x));
        m.insert("crop".into(), json!({ "left": r(c.left), "top": r(c.top), "right": r(c.right), "bottom": r(c.bottom) }));
    }
    Value::Object(m)
}

/// a rendition's `time`: (offset, fps); none when it was never measured
pub fn time(rendition: &Value) -> Option<(f64, f64)> {
    let t = rendition.get("time").filter(|t| t.is_object())?;
    Some((num(t, "offset", 0.0), num(t, "fps", 0.0)))
}

/// The asset of `detail` (`GET /api/assets/{id}`, or `lookup`'s `asset`) and what plays the file `path`: its edits and
/// the time map offset of that file (a file without a measured map: 0, and the warning to show).
pub fn for_file(detail: &Value, path: &str) -> (Edits, f64, Option<String>) {
    let edits = edits(if detail["asset"].is_object() { &detail["asset"]["edits"] } else { &detail["edits"] });
    let renditions = detail["versions"].as_array().into_iter().flatten().filter(|v| v["status"] == "current").flat_map(|v| v["renditions"].as_array().into_iter().flatten());
    let file = renditions.chain(detail["renditions"].as_array().into_iter().flatten()).find(|r| r["path"].as_str() == Some(path));
    match file.and_then(time) {
        Some((offset, _)) => (edits, offset, None),
        None if edits.none() => (edits, 0.0, None),
        None => (edits, 0.0, Some(format!("{path}: no time map measured – taken to start with its source (offset 0)"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edit_list_reads_back_what_was_written() {
        let e = Edits::new(Trim::new(1.5, 4.0), Crop::new(0.1, 0.0, 0.2, 0.05), 0.5, 1.0, 1.25, -6.0);
        let v = to_json(&e);
        assert_eq!(v, json!({ "start": 1.5, "end": 4.0, "fade_in": 0.5, "fade_out": 1.0, "speed": 1.25, "gain_db": -6.0, "crop": { "left": 0.1, "top": 0.0, "right": 0.2, "bottom": 0.05 } }));
        let back = edits(&v);
        assert_eq!((back.trim, back.fade_in, back.speed, back.gain_db), (e.trim, e.fade_in, e.speed, e.gain_db));
        assert!((back.crop.right - 0.2).abs() < 1e-6);
        assert_eq!(to_json(&Edits::NONE), Value::Null);
        assert_eq!(to_json(&Edits { speed: 2.0, ..Edits::NONE }), json!({ "speed": 2.0 }), "only what differs");
    }

    #[test]
    fn what_cannot_be_read_is_no_edit_and_numbers_are_cleaned() {
        assert!(edits(&Value::Null).none());
        assert!(edits(&json!("speed")).none());
        assert!(edits(&json!({ "speed": "fast", "gain_db": null })).none());
        assert_eq!(edits(&json!({ "speed": 99 })).speed, deck_core::asset_edit::SPEED.1);
    }

    #[test]
    fn the_file_played_brings_its_time_map_or_a_warning() {
        let detail = json!({
            "asset": { "edits": { "start": 1.0 } },
            "versions": [{ "status": "current", "renditions": [
                { "path": "v/x.720p.mp4", "time": { "offset": 0.5, "fps": 24.0 } },
                { "path": "v/x.mov" }
            ] }]
        });
        let (e, offset, warn) = for_file(&detail, "v/x.720p.mp4");
        assert_eq!((e.trim.start, offset, warn), (1.0, 0.5, None));
        let (_, offset, warn) = for_file(&detail, "v/x.mov");
        assert_eq!(offset, 0.0);
        assert!(warn.unwrap().contains("no time map"));
        assert_eq!(for_file(&json!({ "versions": [] }), "v/x.mov").2, None, "no edits: nothing to warn about");
    }
}
