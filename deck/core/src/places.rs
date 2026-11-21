//! The places a jump can lead to in a chapter: its groups' ids (#706), in order, each once (a jump `{ chapter, label }` lands
//! at that group). The creator's jump targets and the conductor's label picker read this one list.

use serde_json::Value;

/// a chapter's jump places from its `groups`: every non-empty text id once, in document order; anything else is none
pub fn jump_labels(groups: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for label in groups.iter().filter_map(|g| g.get("id")?.as_str()).filter(|l| !l.is_empty()) {
        if !out.iter().any(|o| o == label) {
            out.push(label.to_string());
        }
    }
    out
}

/// a chapter document's jump places (`{ groups: [...] }`); none for a document without groups
pub fn chapter_labels(chapter: &Value) -> Vec<String> {
    chapter.get("groups").and_then(Value::as_array).map_or_else(Vec::new, |g| jump_labels(g))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn labels_in_order_each_once_unlabelled_and_empty_ones_are_none() {
        let groups = [json!({ "id": "start" }), json!({}), json!({ "id": "" }), json!({ "id": "harbour" }), json!({ "id": "start" }), Value::Null, json!({ "id": 3 })];
        assert_eq!(jump_labels(&groups), ["start", "harbour"]);
        assert!(jump_labels(&[]).is_empty());
    }

    #[test]
    fn a_chapter_without_groups_has_no_places() {
        assert_eq!(chapter_labels(&json!({ "groups": [{ "id": "a" }, { "id": "b" }] })), ["a", "b"]);
        assert!(chapter_labels(&json!({ "groups": "x" })).is_empty());
        assert!(chapter_labels(&Value::Null).is_empty());
    }
}
