//! The node view's own part of a row: groups (nodes framed with a label and a colour, folded to one pill), notes (a
//! comment with its author, beside a node) and the nodes folded or opened by hand. Edited like the rows (rows.rs).
use crate::document::{Cell, Show};
use crate::rows::Edited;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// a group's colours, in the order new groups take them
pub const COLOURS: [&str; 6] = ["#5fb3ff", "#7fd68a", "#f2b84b", "#e07ad6", "#ff7a6b", "#9a8cff"];
/// a folded group's pill in the graph: `group:<id>`
pub const GROUP: &str = deck_kit::base::graph::PILL;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Canvas {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
    /// a node folded (true) or opened (false) by hand, by its id; none: as it comes (a kept cell folded)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub folded: BTreeMap<String, bool>,
}

impl Canvas {
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty() && self.notes.is_empty() && self.folded.is_empty()
    }

    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub label: String,
    /// `#rrggbb`
    pub colour: String,
    /// the node ids it frames
    pub nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub folded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub text: String,
    /// who wrote it (the member's name)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// the node it stands beside; none: after the graph
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// a node folded or opened; `group:<id>` folds or opens that group
    Fold { node: String },
    /// the nodes framed in a new group (each leaves the group it was in)
    Group { nodes: Vec<String> },
    /// the group's label; empty: the group goes, its nodes stay
    GroupLabel { group: String, to: String },
    /// the group takes the next colour
    GroupColour { group: String },
    /// a new empty note by `author`, beside `node`
    Note { node: Option<String>, author: String },
    /// the note's text; empty: the note goes
    NoteText { note: String, to: String },
}

/// whether a node stands folded when nobody folded or opened it: a kept cell (or an outlet the row leaves out)
pub fn folded_as_it_comes(show: &Show, row: &crate::document::Row, node: &str) -> bool {
    show.outlet(node).is_some() && matches!(row.cells.get(node), None | Some(Cell::Keep))
}

/// whether `node` stands folded in `row`
pub fn is_folded(show: &Show, row: &crate::document::Row, node: &str) -> bool {
    row.nodes.folded.get(node).copied().unwrap_or_else(|| folded_as_it_comes(show, row, node))
}

/// the first free id `<stem>-<n>`
fn free(stem: &str, taken: impl Fn(&str) -> bool) -> String {
    (1..).map(|n| format!("{stem}-{n}")).find(|id| !taken(id)).unwrap_or_default()
}

/// Edit `e` on row `row` of `show`: the new document and what happened, in words.
pub fn edit(show: &Show, row: &str, e: &Edit) -> Result<Edited, String> {
    let mut next = show.clone();
    let at = next.rows.iter().position(|r| r.id == row).ok_or_else(|| format!("no row '{row}'"))?;
    let r = &next.rows[at].clone();
    let c = &mut next.rows[at].nodes;
    let group_at = |c: &Canvas, id: &str| c.groups.iter().position(|g| g.id == id).ok_or_else(|| format!("no group '{id}'"));
    let said = match e {
        Edit::Fold { node } => match node.strip_prefix(GROUP) {
            Some(id) => {
                let i = group_at(c, id)?;
                let g = &mut c.groups[i];
                g.folded = !g.folded;
                format!("group {} {}", g.label, if g.folded { "folded" } else { "opened" })
            }
            None => {
                let to = !is_folded(show, r, node);
                match to == folded_as_it_comes(show, r, node) {
                    true => c.folded.remove(node),
                    false => c.folded.insert(node.clone(), to),
                };
                format!("{node} {}", if to { "folded" } else { "opened" })
            }
        },
        Edit::Group { nodes } => {
            if nodes.is_empty() {
                return Err("no node selected to group".into());
            }
            for g in &mut c.groups {
                g.nodes.retain(|n| !nodes.contains(n));
            }
            c.groups.retain(|g| !g.nodes.is_empty());
            let id = free("group", |id| c.group(id).is_some());
            let colour = COLOURS.iter().find(|k| !c.groups.iter().any(|g| g.colour == **k)).unwrap_or(&COLOURS[c.groups.len() % COLOURS.len()]);
            let label = format!("group {}", c.groups.len() + 1);
            c.groups.push(Group { id, label: label.clone(), colour: (*colour).into(), nodes: nodes.clone(), folded: false });
            format!("{label}: {} grouped", nodes.len())
        }
        Edit::GroupLabel { group, to } => {
            let i = group_at(c, group)?;
            match to.trim() {
                "" => format!("{} ungrouped", c.groups.remove(i).label),
                to => {
                    c.groups[i].label = to.into();
                    format!("group named {to}")
                }
            }
        }
        Edit::GroupColour { group } => {
            let i = group_at(c, group)?;
            let g = &mut c.groups[i];
            let k = COLOURS.iter().position(|k| *k == g.colour).map_or(0, |k| (k + 1) % COLOURS.len());
            g.colour = COLOURS[k].into();
            format!("{} recoloured", g.label)
        }
        Edit::Note { node, author } => {
            let id = free("note", |id| c.notes.iter().any(|n| n.id == id));
            c.notes.push(Note { id, text: String::new(), author: author.clone(), node: node.clone() });
            "note added".into()
        }
        Edit::NoteText { note, to } => {
            let i = c.notes.iter().position(|n| n.id == *note).ok_or_else(|| format!("no note '{note}'"))?;
            match to.trim() {
                "" => {
                    c.notes.remove(i);
                    "note removed".into()
                }
                to => {
                    c.notes[i].text = to.into();
                    "note written".into()
                }
            }
        }
    };
    Ok(Edited { show: next, said, row: Some(row.into()) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse;
    use crate::far::tests::SHOW;

    fn show() -> (Show, String) {
        let s = parse(SHOW).unwrap();
        let row = s.rows[0].id.clone();
        (s, row)
    }

    #[test]
    fn a_node_folds_and_opens_and_only_a_change_is_written() {
        let (s, row) = show();
        let kept = s.outlets.iter().find(|o| !s.rows[0].cells.contains_key(&o.id)).map(|o| o.id.clone());
        let e = edit(&s, &row, &Edit::Fold { node: "row".into() }).unwrap();
        assert_eq!(e.show.rows[0].nodes.folded.get("row"), Some(&true));
        let back = edit(&e.show, &row, &Edit::Fold { node: "row".into() }).unwrap();
        assert!(back.show.rows[0].nodes.is_empty(), "opened again: as it comes, nothing kept");
        if let Some(k) = kept {
            assert!(is_folded(&s, &s.rows[0], &k), "a kept cell comes folded");
            let open = edit(&s, &row, &Edit::Fold { node: k.clone() }).unwrap();
            assert_eq!(open.show.rows[0].nodes.folded.get(&k), Some(&false));
        }
    }

    #[test]
    fn a_group_takes_its_nodes_from_another_and_a_colour_of_its_own() {
        let (s, row) = show();
        let a = edit(&s, &row, &Edit::Group { nodes: vec!["reader".into(), "clock".into()] }).unwrap();
        let b = edit(&a.show, &row, &Edit::Group { nodes: vec!["clock".into(), "row".into()] }).unwrap();
        let c = &b.show.rows[0].nodes;
        assert_eq!(c.groups.len(), 2);
        assert_eq!(c.groups[0].nodes, ["reader"], "the clock left the first group");
        assert_ne!(c.groups[0].colour, c.groups[1].colour);
        assert_ne!(c.groups[0].id, c.groups[1].id);
        let gone = edit(&b.show, &row, &Edit::Group { nodes: vec!["reader".into()] }).unwrap();
        assert_eq!(gone.show.rows[0].nodes.groups.len(), 2, "an emptied group goes");
        assert!(edit(&s, &row, &Edit::Group { nodes: vec![] }).is_err());
    }

    #[test]
    fn a_group_folds_is_named_recoloured_and_ungrouped_by_an_empty_name() {
        let (s, row) = show();
        let a = edit(&s, &row, &Edit::Group { nodes: vec!["reader".into()] }).unwrap().show;
        let id = a.rows[0].nodes.groups[0].id.clone();
        let f = edit(&a, &row, &Edit::Fold { node: format!("{GROUP}{id}") }).unwrap().show;
        assert!(f.rows[0].nodes.groups[0].folded);
        let n = edit(&f, &row, &Edit::GroupLabel { group: id.clone(), to: " sources ".into() }).unwrap().show;
        assert_eq!(n.rows[0].nodes.groups[0].label, "sources");
        let k = edit(&n, &row, &Edit::GroupColour { group: id.clone() }).unwrap().show;
        assert_eq!(k.rows[0].nodes.groups[0].colour, COLOURS[1]);
        let u = edit(&k, &row, &Edit::GroupLabel { group: id, to: "".into() }).unwrap().show;
        assert!(u.rows[0].nodes.groups.is_empty());
    }

    #[test]
    fn a_note_keeps_its_author_and_goes_when_emptied() {
        let (s, row) = show();
        let a = edit(&s, &row, &Edit::Note { node: Some("row".into()), author: "Tilman".into() }).unwrap().show;
        let id = a.rows[0].nodes.notes[0].id.clone();
        let w = edit(&a, &row, &Edit::NoteText { note: id.clone(), to: "louder here".into() }).unwrap().show;
        let n = &w.rows[0].nodes.notes[0];
        assert_eq!((n.text.as_str(), n.author.as_str(), n.node.as_deref()), ("louder here", "Tilman", Some("row")));
        let e = edit(&w, &row, &Edit::NoteText { note: id, to: " ".into() }).unwrap().show;
        assert!(e.rows[0].nodes.notes.is_empty());
    }

    #[test]
    fn the_canvas_is_stored_with_the_row_and_refuses_unknown_fields() {
        let (s, row) = show();
        let a = edit(&s, &row, &Edit::Group { nodes: vec!["reader".into()] }).unwrap().show;
        let a = edit(&a, &row, &Edit::Note { node: None, author: "me".into() }).unwrap().show;
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"nodes\":{\"groups\""), "{json}");
        assert_eq!(parse(&json).unwrap(), a, "read back the same");
        assert!(!serde_json::to_string(&s).unwrap().contains("\"nodes\":"), "an empty canvas is not written");
        let bad = json.replacen("\"folded\"", "\"foldd\"", 1).replacen("\"label\":", "\"lable\":", 1);
        assert!(parse(&bad).is_err(), "an unknown field is refused");
    }
}
