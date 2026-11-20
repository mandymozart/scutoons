//! A document's saved history as steps a person reads, reverts and steps back through (#687, after OSCT's #312): every
//! save is a version; a burst of saves is one step; a change is described as the places it touched, before → after; a
//! revert undoes exactly those places on the current document and keeps every place changed again since.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// saves by the same person closer than this are one step (an autosaved burst)
pub const BURST_GAP_MS: i64 = 10_000;
/// a value is shown up to this many characters
pub const SHOWN_CHARS: usize = 40;
/// a description lists this many places, then counts the rest
pub const SHOWN_PLACES: usize = 30;

/// one saved version as the history lists it
#[derive(Debug, Clone, Deserialize)]
pub struct Saved {
    pub id: i64,
    #[serde(default)]
    pub author: Option<String>,
    pub at_ms: i64,
    /// a note saved with it: such a version is a step of its own
    #[serde(default)]
    pub message: Option<String>,
}

/// versions `first..=last` saved together; `before` = the version they changed (none: the document's first, or older than the list)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Step {
    pub first: i64,
    pub last: i64,
    pub before: Option<i64>,
    pub author: Option<String>,
    pub at_ms: i64,
    pub count: usize,
}

fn noted(s: &Saved) -> bool {
    s.message.as_deref().is_some_and(|m| !m.trim().is_empty())
}

/// the steps of `versions` (newest first, as listed), newest first
pub fn steps(versions: &[Saved]) -> Vec<Step> {
    let mut ordered: Vec<&Saved> = versions.iter().collect();
    ordered.sort_by_key(|s| s.id);
    let mut out: Vec<Step> = Vec::new();
    let mut prev: Option<&Saved> = None;
    for s in ordered {
        let joins = prev.is_some_and(|p| !noted(p) && !noted(s) && p.author == s.author && s.at_ms - p.at_ms <= BURST_GAP_MS);
        match out.last_mut() {
            Some(step) if joins => {
                step.last = s.id;
                step.at_ms = s.at_ms;
                step.count += 1;
            }
            _ => out.push(Step { first: s.id, last: s.id, before: prev.map(|p| p.id), author: s.author.clone(), at_ms: s.at_ms, count: 1 }),
        }
        prev = Some(s);
    }
    out.reverse();
    out
}

/// a save of a document that rides along with the one whose steps are listed (a rig's instrument or preset), by its key
#[derive(Debug, Clone)]
pub struct SideSaved {
    pub key: String,
    pub saved: Saved,
}

/// a side document's save counts as part of a step when it came this long before the step's last save at most
pub const SIDE_WINDOW_MS: i64 = 60_000;

/// the side documents saved with each of `steps` (newest first, as `steps` gives them), as indices into `sides`: by the
/// step's author, after the step before it and before its last save (version ids run over all documents), and not
/// longer than `SIDE_WINDOW_MS` ahead of it
pub fn with_sides(steps: &[Step], sides: &[SideSaved]) -> Vec<Vec<usize>> {
    steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let floor = steps.get(i + 1).map_or(i64::MIN, |older| older.last);
            sides
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    let v = &s.saved;
                    v.id > floor && v.id < step.last && v.author == step.author && v.at_ms <= step.at_ms && step.at_ms - v.at_ms <= SIDE_WINDOW_MS
                })
                .map(|(n, _)| n)
                .collect()
        })
        .collect()
}

/// one place a change touched: `None` = not there
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub path: Vec<String>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// the places that differ between two documents: objects key by key, lists of the same length item by item, anything
/// else as a whole
pub fn changes(before: &Value, after: &Value) -> Vec<Change> {
    let mut out = Vec::new();
    walk(&mut Vec::new(), Some(before), Some(after), &mut out);
    out
}

fn walk(path: &mut Vec<String>, a: Option<&Value>, b: Option<&Value>, out: &mut Vec<Change>) {
    match (a, b) {
        (Some(Value::Object(x)), Some(Value::Object(y))) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))).collect();
            keys.sort();
            for k in keys {
                path.push(k.clone());
                walk(path, x.get(k), y.get(k), out);
                path.pop();
            }
        }
        (Some(Value::Array(x)), Some(Value::Array(y))) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                path.push(i.to_string());
                walk(path, Some(p), Some(q), out);
                path.pop();
            }
        }
        _ if a == b => {}
        _ => out.push(Change { path: path.clone(), before: a.cloned(), after: b.cloned() }),
    }
}

/// how a place changed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Added,
    Removed,
    Changed,
}

/// one line of a description: where, how, the value before and after as shown
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Line {
    pub place: String,
    pub kind: Kind,
    pub before: String,
    pub after: String,
}

/// a change in words: its first places and how many more there are
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Description {
    pub lines: Vec<Line>,
    pub more: usize,
}

/// a place as a person reads it: the keys joined, list items counted from 1
pub fn place(path: &[String]) -> String {
    if path.is_empty() {
        return "·".into();
    }
    path.iter().map(|s| s.parse::<usize>().map_or_else(|_| s.clone(), |i| format!("#{}", i + 1))).collect::<Vec<_>>().join(" › ")
}

/// a value as a person reads it, cut after `SHOWN_CHARS`
pub fn shown(v: Option<&Value>) -> String {
    let text = match v {
        None => return String::new(),
        Some(Value::Null) => "∅".to_string(),
        Some(Value::String(s)) => format!("“{s}”"),
        Some(Value::Array(a)) => format!("[{}]", a.len()),
        Some(Value::Object(o)) => format!("{{{}}}", o.len()),
        Some(other) => other.to_string(),
    };
    if text.chars().count() <= SHOWN_CHARS {
        return text;
    }
    let cut: String = text.chars().take(SHOWN_CHARS - 1).collect();
    format!("{cut}…")
}

/// what changed between two documents, in words
pub fn describe(before: &Value, after: &Value) -> Description {
    let all = changes(before, after);
    let more = all.len().saturating_sub(SHOWN_PLACES);
    let lines = all
        .iter()
        .take(SHOWN_PLACES)
        .map(|c| Line {
            place: place(&c.path),
            kind: match (&c.before, &c.after) {
                (None, _) => Kind::Added,
                (_, None) => Kind::Removed,
                _ => Kind::Changed,
            },
            before: shown(c.before.as_ref()),
            after: shown(c.after.as_ref()),
        })
        .collect();
    Description { lines, more }
}

/// a revert's outcome: the new document, how many places went back, the places changed again since (left as they are)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Revert {
    pub doc: Value,
    pub applied: usize,
    pub kept: Vec<String>,
}

/// undoes the change `before` → `after` on `current`: a place that still holds `after` gets `before` back; a place
/// changed since is kept and named
pub fn revert(before: &Value, after: &Value, current: &Value) -> Revert {
    let mut doc = current.clone();
    let mut applied = 0;
    let mut kept = Vec::new();
    for c in changes(before, after) {
        let now = get(&doc, &c.path).cloned();
        if now == c.before {
            continue;
        }
        if now == c.after && put(&mut doc, &c.path, c.before.clone()) {
            applied += 1;
        } else {
            kept.push(place(&c.path));
        }
    }
    Revert { doc, applied, kept }
}

fn get<'a>(v: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(v, |at, k| match at {
        Value::Object(o) => o.get(k),
        Value::Array(a) => k.parse::<usize>().ok().and_then(|i| a.get(i)),
        _ => None,
    })
}

/// sets (or, with `None`, removes) the value at `path`; false when its parent is not there
fn put(doc: &mut Value, path: &[String], value: Option<Value>) -> bool {
    let Some((last, parents)) = path.split_last() else {
        return match value {
            Some(v) => {
                *doc = v;
                true
            }
            None => false,
        };
    };
    let mut at = doc;
    for k in parents {
        at = match at {
            Value::Object(o) => match o.get_mut(k) {
                Some(next) => next,
                None => return false,
            },
            Value::Array(a) => match k.parse::<usize>().ok().and_then(|i| a.get_mut(i)) {
                Some(next) => next,
                None => return false,
            },
            _ => return false,
        };
    }
    match (at, value) {
        (Value::Object(o), Some(v)) => {
            o.insert(last.clone(), v);
            true
        }
        (Value::Object(o), None) => o.remove(last).is_some(),
        (Value::Array(a), Some(v)) => match last.parse::<usize>().ok().and_then(|i| a.get_mut(i)) {
            Some(slot) => {
                *slot = v;
                true
            }
            None => false,
        },
        _ => false,
    }
}

/// what going back to version `point` undoes: the saves after it and the other people among them
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackTo {
    pub later: usize,
    pub others: Vec<String>,
}

/// the prompt's numbers for going back to `point`; `me` = the person asking
pub fn back_to(versions: &[Saved], point: i64, me: &str) -> BackTo {
    let after: Vec<&Saved> = versions.iter().filter(|s| s.id > point).collect();
    let mut others: Vec<String> = after.iter().filter_map(|s| s.author.clone()).filter(|a| a != me).collect();
    others.sort();
    others.dedup();
    BackTo { later: after.len(), others }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn saved(id: i64, author: &str, at_ms: i64) -> Saved {
        Saved { id, author: Some(author.into()), at_ms, message: None }
    }

    #[test]
    fn a_burst_by_one_person_is_one_step() {
        let v = [saved(4, "b", 40_000), saved(3, "a", 25_000), saved(2, "a", 9_000), saved(1, "a", 0)];
        let s = steps(&v);
        assert_eq!(s.len(), 3);
        assert_eq!((s[0].first, s[0].last, s[0].before), (4, 4, Some(3)));
        assert_eq!((s[1].first, s[1].last, s[1].before), (3, 3, Some(2)));
        assert_eq!((s[2].first, s[2].last, s[2].before, s[2].count), (1, 2, None, 2));
    }

    #[test]
    fn a_noted_save_stands_alone() {
        let mut v = vec![saved(1, "a", 0), saved(2, "a", 1_000)];
        v[1].message = Some("revert #1".into());
        assert_eq!(steps(&v).len(), 2);
    }

    #[test]
    fn describes_places_before_and_after() {
        let d = describe(&json!({"title": {"de": "Alt"}, "list": [1, 2]}), &json!({"title": {"de": "Neu", "en": "New"}, "list": [1, 3]}));
        let places: Vec<_> = d.lines.iter().map(|l| (l.place.as_str(), l.kind, l.before.as_str(), l.after.as_str())).collect();
        assert_eq!(places, vec![("list › #2", Kind::Changed, "2", "3"), ("title › de", Kind::Changed, "“Alt”", "“Neu”"), ("title › en", Kind::Added, "", "“New”")]);
        assert_eq!(d.more, 0);
    }

    #[test]
    fn a_long_value_is_cut() {
        let s = shown(Some(&json!("x".repeat(100))));
        assert_eq!(s.chars().count(), SHOWN_CHARS);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn revert_undoes_only_its_places_and_keeps_later_ones() {
        let before = json!({"a": 1, "b": 1, "c": {"x": 1}});
        let after = json!({"a": 2, "b": 2, "c": {"x": 1, "y": 1}});
        let current = json!({"a": 2, "b": 5, "c": {"x": 9, "y": 1}, "d": true});
        let r = revert(&before, &after, &current);
        assert_eq!(r.doc, json!({"a": 1, "b": 5, "c": {"x": 9}, "d": true}));
        assert_eq!(r.applied, 2);
        assert_eq!(r.kept, vec!["b".to_string()]);
    }

    #[test]
    fn reverting_a_revert_redoes() {
        let v1 = json!({"t": "one"});
        let v2 = json!({"t": "two"});
        let undone = revert(&v1, &v2, &v2).doc;
        assert_eq!(undone, v1);
        assert_eq!(revert(&v2, &undone, &undone).doc, v2);
    }

    #[test]
    fn a_list_of_another_length_is_one_place() {
        let r = revert(&json!({"l": [1]}), &json!({"l": [1, 2]}), &json!({"l": [1, 2]}));
        assert_eq!(r.doc, json!({"l": [1]}));
        let kept = revert(&json!({"l": [1]}), &json!({"l": [1, 2]}), &json!({"l": [1, 2, 3]}));
        assert_eq!(kept.kept, vec!["l".to_string()]);
    }

    #[test]
    fn side_documents_belong_to_the_step_that_saved_them() {
        let v = [saved(12, "a", 61_000), saved(8, "a", 30_000), saved(5, "a", 0)];
        let steps = steps(&v);
        assert_eq!(steps.iter().map(|s| s.last).collect::<Vec<_>>(), [12, 8, 5]);
        let side = |key: &str, id: i64, who: &str, at_ms: i64| SideSaved { key: key.into(), saved: saved(id, who, at_ms) };
        let sides = [
            side("instrument:bell", 11, "a", 60_500),
            side("preset:soft", 10, "a", 60_400),
            side("instrument:bell", 7, "a", 29_900),
            side("instrument:bell", 9, "b", 60_000),
            side("instrument:bell", 6, "a", -90_000),
            side("instrument:bell", 13, "a", 62_000),
        ];
        let got = with_sides(&steps, &sides);
        assert_eq!(got, vec![vec![0, 1], vec![2], vec![]], "after the step before, before its last save, its author, a minute ahead at most");
    }

    #[test]
    fn back_to_counts_later_saves_and_other_people() {
        let v = [saved(5, "b", 0), saved(4, "a", 0), saved(3, "b", 0), saved(2, "a", 0)];
        assert_eq!(back_to(&v, 2, "a"), BackTo { later: 3, others: vec!["b".into()] });
        assert_eq!(back_to(&v, 5, "a"), BackTo { later: 0, others: vec![] });
    }
}
