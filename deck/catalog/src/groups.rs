//! A module's parameters in groups: what a node shows as one cluster each until it is opened. A descriptor carries no
//! sections, so the rule is the names': the part before the first `_`, its trailing number dropped (`o1_wave` and
//! `o2_wave` are one group). A group needs a few members; the rest stay loose.

/// the fewest parameters a group holds; fewer stay loose
pub const MIN_GROUP: usize = 3;

/// One group: its name as shown (the prefixes it joins, `o1/o2`) and its parameters in their order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub params: Vec<String>,
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
    stems.into_iter().filter(|(_, _, params)| params.len() >= MIN_GROUP).map(|(_, prefixes, params)| Group { name: prefixes.join("/"), params }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_prefixes_are_one_group_and_small_ones_stay_loose() {
        let names = ["volume", "o1_wave", "o1_level", "noise_level", "o2_wave", "o2_level", "noise_route", "f_serial", "f1_cutoff", "f2_cutoff"];
        let g = by_prefix(&names);
        assert_eq!(g, vec![Group { name: "o1/o2".into(), params: vec!["o1_wave".into(), "o1_level".into(), "o2_wave".into(), "o2_level".into()] }, Group { name: "f/f1/f2".into(), params: vec!["f_serial".into(), "f1_cutoff".into(), "f2_cutoff".into()] }]);
    }

    #[test]
    fn no_prefixes_no_groups() {
        assert!(by_prefix(&["gain", "pan", "rate", "_x", "x_"]).is_empty());
    }
}
