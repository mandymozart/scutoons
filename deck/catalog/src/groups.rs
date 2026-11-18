//! A module's parameters in groups: what a node shows as one cluster each until it is opened. A module with `sections`
//! groups by them: an area is a group, its pages in order, each page's knobs by slot. Without, the rule is the names':
//! the part before the first `_`, its trailing number dropped (`o1_wave` and `o2_wave` are one group); such a group
//! needs a few members, the rest stay loose.
use crate::{Descriptor, PAGE_SLOTS};

/// the fewest parameters a group holds; fewer stay loose
pub const MIN_GROUP: usize = 3;

/// One group: its name as shown (an area, `SRC`; or the prefixes it joins, `o1/o2`), its parameters in their order
/// and, from sections, its pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub params: Vec<String>,
    pub pages: Vec<Page>,
}

/// One page of an area: its name and the parameter on each of its eight knobs (two rows of four).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub name: String,
    pub slots: [Option<String>; PAGE_SLOTS],
}

/// A descriptor's parameters by its sections: an area per group in the order its first page comes, its parameters
/// page by page and knob by knob. A parameter on an undeclared page or knob (a descriptor `check` refuses) is left out.
pub fn by_sections(d: &Descriptor) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for s in d.sections.iter() {
        let key = s.key();
        let mut slots: [Option<String>; PAGE_SLOTS] = Default::default();
        for p in d.params.iter().filter(|p| p.section == key) {
            if let Some(slot) = p.slot.map(usize::from).filter(|&k| k < PAGE_SLOTS) {
                slots[slot].get_or_insert_with(|| p.name.to_string());
            }
        }
        let page = Page { name: s.page.to_string(), slots };
        match groups.iter_mut().find(|g| g.name == s.area) {
            Some(g) => g.pages.push(page),
            None => groups.push(Group { name: s.area.to_string(), params: Vec::new(), pages: vec![page] }),
        }
    }
    for g in &mut groups {
        g.params = g.pages.iter().flat_map(|p| p.slots.iter().flatten().cloned()).collect();
    }
    groups
}

/// `names` in groups by the prefix rule, in the order each group's first parameter comes.
pub fn by_prefix<S: AsRef<str>>(names: &[S]) -> Vec<Group> {
    let mut stems: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
    for name in names.iter().map(AsRef::as_ref) {
        let Some((prefix, _)) = name.split_once('_').filter(|(p, _)| !p.is_empty()) else { continue };
        let stem = prefix.trim_end_matches(|c: char| c.is_ascii_digit());
        let stem = if stem.is_empty() { prefix } else { stem };
        let at = match stems.iter().position(|(s, ..)| s == stem) {
            Some(at) => at,
            None => {
                stems.push((stem.to_string(), Vec::new(), Vec::new()));
                stems.len() - 1
            }
        };
        let (_, prefixes, params) = &mut stems[at];
        if !prefixes.iter().any(|p| p == prefix) {
            prefixes.push(prefix.to_string());
        }
        params.push(name.to_string());
    }
    stems.into_iter().filter(|(_, _, params)| params.len() >= MIN_GROUP).map(|(_, prefixes, params)| Group { name: prefixes.join("/"), params, pages: Vec::new() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_prefixes_are_one_group_and_small_ones_stay_loose() {
        let names = ["volume", "o1_wave", "o1_level", "noise_level", "o2_wave", "o2_level", "noise_route", "f_serial", "f1_cutoff", "f2_cutoff"];
        let g = by_prefix(&names);
        assert_eq!(g, vec![Group { name: "o1/o2".into(), params: vec!["o1_wave".into(), "o1_level".into(), "o2_wave".into(), "o2_level".into()], pages: vec![] }, Group { name: "f/f1/f2".into(), params: vec!["f_serial".into(), "f1_cutoff".into(), "f2_cutoff".into()], pages: vec![] }]);
    }

    fn synth(sections: serde_json::Value, params: serde_json::Value) -> Descriptor {
        serde_json::from_value(serde_json::json!({ "name": "s", "sections": sections, "params": params })).unwrap()
    }

    /// sections come first: an area per group in page order, knobs by slot, twin pages on the same knobs
    #[test]
    fn sections_group_by_area_and_page_before_the_prefix_rule() {
        let d = synth(
            serde_json::json!([{ "area": "SRC", "page": "OSC 1" }, { "area": "FLTR", "page": "FILTER 1" }, { "area": "SRC", "page": "OSC 2" }]),
            serde_json::json!([
                { "name": "o2_level", "max": 1, "section": "SRC/OSC 2", "slot": 4 },
                { "name": "o1_wave", "max": 1, "section": "SRC/OSC 1", "slot": 0 },
                { "name": "o1_level", "max": 1, "section": "SRC/OSC 1", "slot": 4 },
                { "name": "f1_cutoff", "max": 1, "section": "FLTR/FILTER 1", "slot": 0 },
                { "name": "o2_sync", "max": 1, "section": "SRC/OSC 2", "slot": 7 },
            ]),
        );
        d.check().unwrap();
        let g = d.param_groups();
        assert_eq!(g.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["SRC", "FLTR"], "areas, not prefixes");
        assert_eq!(g[0].params, ["o1_wave", "o1_level", "o2_level", "o2_sync"], "page by page, knob by knob");
        assert_eq!(g[0].pages.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["OSC 1", "OSC 2"]);
        assert_eq!((g[0].pages[0].slots[4].as_deref(), g[0].pages[1].slots[4].as_deref()), (Some("o1_level"), Some("o2_level")), "twins share a knob");
        assert_eq!(g[0].pages[0].slots[7], None);
        assert_eq!(g[1].params, ["f1_cutoff"], "a small area is still a group");
    }

    #[test]
    fn sections_are_checked() {
        let ok = || synth(serde_json::json!([{ "area": "A", "page": "P" }, { "area": "A", "page": "Q" }]), serde_json::json!([{ "name": "x", "max": 1, "section": "A/P", "slot": 0 }, { "name": "y", "max": 1, "section": "A/Q", "slot": 0 }]));
        ok().check().unwrap();
        let bad = |f: &dyn Fn(&mut Descriptor)| {
            let mut d = ok();
            f(&mut d);
            d.check().unwrap_err()
        };
        assert!(bad(&|d| d.params.to_mut()[1].slot = Some(8)).contains("0 … 7"));
        assert!(bad(&|d| d.params.to_mut()[1].section = "A/P".into()).contains("taken"));
        assert!(bad(&|d| d.params.to_mut()[1].section = "A/R".into()).contains("not declared"));
        assert!(bad(&|d| d.params.to_mut()[1].section = "".into()).contains("on no page"));
        assert!(bad(&|d| d.params.to_mut()[1].slot = None).contains("no `slot`"));
        assert!(bad(&|d| d.sections.to_mut()[1].page = "P".into()).contains("twice"));
        assert!(bad(&|d| d.sections.to_mut()[1].page = "a/b".into()).contains("without '/'"));
        assert!(bad(&|d| d.sections.to_mut().push(crate::Section { area: "B".into(), page: "P".into() })).contains("holds no parameter"));
        let mut plain = ok();
        plain.sections = Default::default();
        assert!(plain.check().unwrap_err().contains("not declared"), "a page names a declared section");
        for p in plain.params.to_mut() {
            (p.section, p.slot) = ("".into(), None);
        }
        plain.check().unwrap();
        plain.params.to_mut()[0].slot = Some(0);
        assert!(plain.check().unwrap_err().contains("needs a `section`"));
    }

    #[test]
    fn no_prefixes_no_groups() {
        assert!(by_prefix(&["gain", "pan", "rate", "_x", "x_"]).is_empty());
    }
}
