//! A model's spots as the API carries them (`GET /api/assets/{id}`: `spots` on each version; `PUT` takes `spots` for
//! the current one): read into deck-catalog's `Spots` and written back. The server stores what the core wrote and
//! asks the core before it does (`/validate` kind `spots`); what cannot be read is no spots.
use deck_catalog::spot::Spots;
use serde_json::Value;

/// a version's `spots` (absent, null or not such a document: none)
pub fn spots(v: &Value) -> Spots {
    serde_json::from_value(v.clone()).unwrap_or_default()
}

/// the `spots` to store (nothing: null)
pub fn to_json(s: &Spots) -> Value {
    if s.none() {
        return Value::Null;
    }
    // through text: an f32 is written as the short number it reads back from
    serde_json::to_string(s).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(Value::Null)
}

/// The spots of the current version in `detail` (`GET /api/assets/{id}`).
pub fn current(detail: &Value) -> Spots {
    let version = detail["versions"].as_array().into_iter().flatten().find(|v| v["status"] == "current");
    version.map(|v| spots(&v["spots"])).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_catalog::spot::{Attach, File, Hand, Spot};
    use serde_json::json;

    #[test]
    fn the_spots_read_back_what_was_written() {
        let s = Spots {
            file: Some(File { nodes: vec![], bones: vec!["head".into()], spots: vec![Spot { at: [0.1, 0.6, 0.0], ..Spot::new("flame") }] }),
            hand: vec![Hand { name: "ear".into(), attach: Some(Attach::Bone("head".into())), ..Hand::default() }],
        };
        let v = to_json(&s);
        assert_eq!(v["file"]["spots"][0]["at"], json!([0.1, 0.6, 0.0]), "short numbers");
        assert_eq!(v["hand"], json!([{ "name": "ear", "attach": "bone:head" }]));
        assert_eq!(spots(&v), s);
        assert_eq!(to_json(&Spots::default()), Value::Null);
    }

    #[test]
    fn the_current_version_s_spots_and_none_for_what_cannot_be_read() {
        let detail = json!({ "versions": [
            { "status": "current", "spots": { "hand": [{ "name": "ear" }] } },
            { "status": "old", "spots": { "hand": [{ "name": "old" }] } }
        ] });
        assert_eq!(current(&detail).resolved()[0].name, "ear");
        assert!(current(&json!({ "versions": [{ "status": "current" }] })).none());
        assert!(spots(&json!({ "hand": "ear" })).none() && spots(&Value::Null).none());
    }
}
