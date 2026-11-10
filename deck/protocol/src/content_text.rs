// GENERATED from server/schema/tables/locale-order.json by server/schema/generate.ts – edit the source, run `npm run generate`.
//! The one locale order for content text and its pick (#477).

/// A step of the locale order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// the asked locale
    Asked,
    /// the document's source language
    Source,
    /// this locale
    Locale(&'static str),
    /// the first text in the map's own order (a `BTreeMap`: sorted)
    Any,
}

/// The one locale order for content text (#474, #477): the asked locale › the document's source language › English › any. `asked`, `source` and `any` are steps, anything else a locale code. A text counts when it is a string with more than white space; `any` = the first such text in the map's own order. Generated into Rust, TS and PHP with the one pick function each by server/schema/generate.ts; locale-order.vectors.json is the cases every language runs.
pub const LOCALE_ORDER: [Step; 4] = [Step::Asked, Step::Source, Step::Locale("en"), Step::Any];

/// Content text in the one locale order: the first step with a text (more than white space), none when none has one.
pub fn pick_text<'a, M>(text: M, asked: &str, source: Option<&str>) -> Option<&'a str>
where
    M: IntoIterator<Item = (&'a String, &'a String)> + Copy,
{
    let present = |v: &str| !v.trim().is_empty();
    for step in LOCALE_ORDER {
        let locale = match step {
            Step::Asked => Some(asked),
            Step::Source => source,
            Step::Locale(l) => Some(l),
            Step::Any => return text.into_iter().find(|(_, v)| present(v.as_str())).map(|(_, v)| v.as_str()),
        };
        if let Some(l) = locale {
            if let Some((_, v)) = text.into_iter().find(|(k, v)| k.as_str() == l && present(v.as_str())) {
                return Some(v.as_str());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    /// the cases every language runs (server/schema/tables/locale-order.vectors.json)
    #[test]
    fn the_shared_cases() {
        let file: serde_json::Value = serde_json::from_str(include_str!("../../../server/schema/tables/locale-order.vectors.json")).unwrap();
        for case in file["cases"].as_array().unwrap() {
            let text: BTreeMap<String, String> = serde_json::from_value(case["text"].clone()).unwrap();
            let picked = super::pick_text(&text, case["asked"].as_str().unwrap(), case["source"].as_str());
            assert_eq!(picked, case["expect"].as_str(), "{case}");
        }
    }
}
