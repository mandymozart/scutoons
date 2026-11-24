//! The editor comes up where a `scutoons-deck://<workspace>[/<address>]` link says (DECK-LINK.md, deck-ui `link`): the
//! one it was started with, on its first frame, and every one a later start handed over (instance.rs) – the item opens
//! in its tab (or comes forward), else the workspace does; the window comes forward. And it keeps its device row.
use crate::{tabs, App};
use deck_ui::link;

/// what a link asks of the editor
#[derive(Debug, PartialEq, Eq)]
pub enum Opening {
    /// only come forward (a start without a link)
    Forward,
    /// this item, in its tab
    Item(String),
    /// this workspace, on its last item or empty; a note why the item did not open
    Workspace(deck_ui::workspaces::Workspace, Option<String>),
    /// not a link the deck takes: why
    Refused(String),
}

/// a handed link, read
pub fn opening(text: &str) -> Opening {
    if text.is_empty() {
        return Opening::Forward;
    }
    let Some(l) = link::read(text).filter(link::AppLink::is_deck) else { return Opening::Refused(format!("not a deck link: {text}")) };
    let Some(w) = link::workspace_of(&l.workspace) else { return Opening::Refused(format!("the deck has no workspace {}", l.workspace)) };
    match l.address {
        None => Opening::Workspace(w, None),
        Some(a) if tabs::Kind::of(&a).is_some() => Opening::Item(a),
        Some(a) => Opening::Workspace(w, Some(format!("{a} does not open in the deck yet"))),
    }
}

impl App {
    /// each frame: the start's link (the first frame), the links handed since, the device row
    pub(crate) fn take_links(&mut self) {
        let mut links: Vec<String> = self.args.link.take().into_iter().collect();
        links.extend(self.instance.as_ref().map(crate::instance::Instance::take).unwrap_or_default());
        for l in links {
            self.follow_link(&l);
        }
        if self.autosaves {
            let front = (self.deck_ws(), self.tabs.front().map(|t| t.key.clone()));
            let covered = self.switcher_shows();
            self.row.follow(&self.args.api, self.auth.token(), front, &self.session.uses.last, covered, self.frames);
        }
    }

    fn follow_link(&mut self, text: &str) {
        let o = opening(text);
        if let Some(w) = self.window.as_ref() {
            w.set_minimized(false);
            w.focus_window();
        }
        if !matches!(o, Opening::Forward | Opening::Refused(_)) {
            (self.switcher, self.main_menu) = (false, None);
        }
        match o {
            Opening::Forward => {}
            Opening::Item(a) => self.open_item(&a, &tabs::name_of(&a)),
            Opening::Workspace(w, why) => {
                self.workspace_click(w);
                if let Some(why) = why {
                    self.note(why);
                }
            }
            Opening::Refused(why) => self.note(why),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_ui::workspaces::Workspace;

    #[test]
    fn a_link_opens_its_item_else_its_workspace() {
        assert_eq!(opening("scutoons-deck://stage/scene:nave"), Opening::Item("scene:nave".into()));
        assert_eq!(opening("SCUTOONS-Deck://asset/image:catpope/m%C3%BCller.webp"), Opening::Item("image:catpope/müller.webp".into()));
        assert_eq!(opening("scutoons-deck://show"), Opening::Workspace(Workspace::Show, None));
        assert_eq!(opening("scutoons-deck://stage/"), Opening::Workspace(Workspace::Scene, None));
        assert!(matches!(opening("scutoons-deck://rigging/rig:catpope"), Opening::Workspace(Workspace::Rigging, Some(_))), "no rig tab in the deck yet");
        assert!(matches!(opening("scutoons-deck://stage/shot:catpope:3:sb-25-1"), Opening::Workspace(Workspace::Scene, Some(_))), "a shot is no tab yet: its workspace comes up");
        assert_eq!(opening(""), Opening::Forward);
        assert!(matches!(opening("scutoons-creator://scene/scene:x"), Opening::Refused(_)), "the creator's link is the creator's");
        assert!(matches!(opening("scutoons-deck://xr/xr:poster"), Opening::Refused(_)));
        assert!(matches!(opening("scutoons-deck://stage/scene:a?b"), Opening::Refused(_)));
    }
}
