//! The deck editor's workspaces (the creator's workspaces.ts, the same rules): Scene · Rigging · Show, one flat row,
//! most used first, never grouped. An open item fits one workspace by its address' kind; a workspace the front item
//! does not fit is dimmed in the row, and clicking it brings its own last item forward (or an open one of its kind),
//! else it opens empty – nothing open there yet. Plain data; the editor keeps it with its resume state.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Workspace {
    Scene,
    Rigging,
    Show,
}

/// the row's order before any use. XR is the creator's workspace only: the deck has no XR features (decisions/deck.md
/// 2026-11-23 [workspaces])
pub const ALL: [Workspace; 3] = [Workspace::Scene, Workspace::Rigging, Workspace::Show];

impl Workspace {
    pub fn word(self) -> &'static str {
        match self {
            Workspace::Scene => "Scene",
            Workspace::Rigging => "Rigging",
            Workspace::Show => "Show",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Workspace::Scene => "map",
            Workspace::Rigging => "tree",
            Workspace::Show => "list",
        }
    }

    /// what it is for: the dashboard's card and the info
    pub fn about(self) -> &'static str {
        match self {
            Workspace::Scene => "The map: holders, camera and shots, events.",
            Workspace::Rigging => "One entity on its own stage: a character with its rig, an item, a module, a medium.",
            Workspace::Show => "Authoring and running a show: rows, cells, GO, the session and the vote.",
        }
    }

    /// the workspace an address' kind fits (`scene:nave` → Scene, `character:catpope` → Rigging); none: no workspace
    pub fn of(key: &str) -> Option<Workspace> {
        let (word, _) = key.split_once(':')?;
        Some(match word {
            "scene" => Workspace::Scene,
            "character" | "item" | "emotion" | "module" | "video" | "audio" | "image" | "model" | "media" => Workspace::Rigging,
            "show" => Workspace::Show,
            _ => return None,
        })
    }
}

/// how often each workspace came to the front, and the item each showed last
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Use {
    #[serde(default)]
    pub uses: BTreeMap<Workspace, u32>,
    #[serde(default)]
    pub last: BTreeMap<Workspace, String>,
}

impl Use {
    /// The front changed (`from`: the workspace in front before) to `to`, showing `key` (none: the workspace empty): it
    /// remembers the item and, entered anew, counts a use
    pub fn entered(&mut self, from: Option<Workspace>, to: Workspace, key: Option<&str>) {
        if from != Some(to) {
            *self.uses.entry(to).or_default() += 1;
        }
        if let Some(k) = key.filter(|k| Workspace::of(k) == Some(to) && !k.ends_with(':')) {
            self.last.insert(to, k.to_string());
        }
    }

    /// The row: the four workspaces, most used first (ties: their own order)
    pub fn row(&self) -> Vec<Workspace> {
        let mut row = ALL.to_vec();
        row.sort_by_key(|w| std::cmp::Reverse(self.uses.get(w).copied().unwrap_or(0)));
        row
    }

    /// nothing ever used: the editor's very first open
    pub fn fresh(&self) -> bool {
        self.uses.is_empty() && self.last.is_empty()
    }
}

/// what a click on a workspace in the row does
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Click {
    /// the front item fits it: it stays, the workspace shows it
    Keep,
    /// this open item comes to the front (its last one, else one of its kind)
    Front(String),
    /// nothing of it is open: it opens empty
    Empty,
}

/// a workspace clicked with `front` in front and `open` the items open
pub fn click(u: &Use, front: Option<&str>, open: &[&str], target: Workspace) -> Click {
    if front.is_some_and(|f| Workspace::of(f) == Some(target)) {
        return Click::Keep;
    }
    let last = u.last.get(&target).filter(|k| open.contains(&k.as_str()));
    match last.cloned().or_else(|| open.iter().find(|k| Workspace::of(k) == Some(target)).map(|k| k.to_string())) {
        Some(k) => Click::Front(k),
        None => Click::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_fits_one_workspace() {
        assert_eq!(Workspace::of("scene:nave"), Some(Workspace::Scene));
        for k in ["character:catpope", "item:lamp", "module:analog", "video:a/b.mov", "emotion:happy"] {
            assert_eq!(Workspace::of(k), Some(Workspace::Rigging), "{k}");
        }
        assert_eq!(Workspace::of("show:duet"), Some(Workspace::Show));
        assert_eq!(Workspace::of("xr:poster"), None, "XR is the creator's: no deck workspace");
        assert_eq!(Workspace::of("catpope"), None);
    }

    #[test]
    fn the_row_is_flat_most_used_first() {
        let mut u = Use::default();
        assert_eq!(u.row(), ALL, "before any use: Scene · Rigging · Show");
        assert!(u.fresh());
        u.entered(None, Workspace::Show, Some("show:duet"));
        u.entered(Some(Workspace::Show), Workspace::Rigging, Some("character:catpope"));
        u.entered(Some(Workspace::Rigging), Workspace::Show, Some("show:duet"));
        u.entered(Some(Workspace::Show), Workspace::Show, Some("show:trio"));
        assert_eq!(u.uses[&Workspace::Show], 2, "staying in a workspace counts no use");
        assert_eq!(u.row(), [Workspace::Show, Workspace::Rigging, Workspace::Scene]);
        assert_eq!(u.last[&Workspace::Show], "show:trio");
        assert!(!u.fresh());
    }

    #[test]
    fn a_click_keeps_the_front_item_brings_the_last_one_or_opens_empty() {
        let mut u = Use::default();
        u.entered(None, Workspace::Scene, Some("scene:nave"));
        u.entered(Some(Workspace::Scene), Workspace::Rigging, Some("character:catpope"));
        let open = ["scene:nave", "scene:crypt", "character:catpope"];
        assert_eq!(click(&u, Some("character:catpope"), &open, Workspace::Rigging), Click::Keep);
        assert_eq!(click(&u, Some("character:catpope"), &open, Workspace::Scene), Click::Front("scene:nave".into()));
        assert_eq!(click(&Use::default(), Some("character:catpope"), &open, Workspace::Scene), Click::Front("scene:nave".into()), "an open one of its kind");
        assert_eq!(click(&u, None, &open, Workspace::Show), Click::Empty);
        assert_eq!(click(&u, Some("scene:nave"), &["scene:nave"], Workspace::Rigging), Click::Empty, "its last one was closed");
    }

    #[test]
    fn use_survives_json() {
        let mut u = Use::default();
        u.entered(None, Workspace::Show, None);
        let back: Use = serde_json::from_str(&serde_json::to_string(&u).unwrap()).unwrap();
        assert_eq!(back, u);
        let old: Use = serde_json::from_str("{}").unwrap();
        assert!(old.fresh());
    }
}
