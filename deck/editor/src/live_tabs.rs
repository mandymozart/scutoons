//! One live show in the editor (decisions/deck.md 2026-11-18 [workspaces]): Go live on a Show tab joins its show to the
//! relay as its conductor; while another show is live (a tab behind holds it), the editor asks first and ends that one.
//! A live show behind keeps running – its conductor hears the relay and keeps its clock every frame. End leaves the relay,
//! Lock refuses the document's edits (show/live.rs).
use crate::tab_view::Kept;

/// a press on a Show tab's live switch in the toolbar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Go,
    End,
    Lock,
}

/// the switch's words, the one lit (none: `usize::MAX`) and what each press means
pub fn switch(live: bool, locked: bool) -> (&'static [&'static str], usize, &'static [Press]) {
    match (live, locked) {
        (false, _) => (&["Go live"], usize::MAX, &[Press::Go]),
        (true, false) => (&["End", "Lock"], usize::MAX, &[Press::End, Press::Lock]),
        (true, true) => (&["End", "Locked"], 1, &[Press::End, Press::Lock]),
    }
}

impl crate::App {
    /// the relay Go live joins: the one named at the start, else the hosted one
    fn go_live_relay(&self) -> String {
        self.args.relay.clone().unwrap_or_else(|| deck_space::relay::HOSTED_RELAY.to_string())
    }

    /// the show live in a tab behind, if any: its id
    fn live_elsewhere(&self) -> Option<String> {
        self.tabs.list.iter().find_map(|t| match &t.kept {
            Some(Kept::Show(h)) if h.live() => Some(h.sub.conductor().show().id.clone()),
            _ => None,
        })
    }

    pub(crate) fn live_press(&mut self, p: Press) {
        match p {
            Press::Go => match self.live_elsewhere() {
                Some(other) => self.show.ask_end(other),
                None => {
                    let relay = self.go_live_relay();
                    self.show.go_live(&relay);
                }
            },
            Press::End => self.show.end_live(),
            Press::Lock => self.show.flip_lock(),
        }
    }

    /// Each frame: an ask confirmed ends the other live show and this one goes live; a live show behind runs on.
    pub(crate) fn follow_live(&mut self) {
        if self.show.take_live_asked() {
            for t in &mut self.tabs.list {
                if let Some(Kept::Show(h)) = t.kept.as_mut() {
                    h.end_live();
                }
            }
            let relay = self.go_live_relay();
            self.show.go_live(&relay);
        }
        for t in &mut self.tabs.list {
            if let Some(Kept::Show(h)) = t.kept.as_mut() {
                if h.live() {
                    h.frame();
                }
            }
        }
    }

    /// a check run's `--go-live A,B`: each show's tab comes forward and Go live is pressed (the second asks); `--lock`
    /// then locks the one in front
    pub(crate) fn script_live(&mut self) {
        for id in self.args.go_live.clone() {
            self.open_item(&format!("show:{id}"), &id);
            self.live_press(Press::Go);
        }
        if self.args.lock {
            self.live_press(Press::Lock);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_says_go_live_then_end_and_lock() {
        assert_eq!(switch(false, false).0, ["Go live"]);
        assert_eq!(switch(true, false), (&["End", "Lock"][..], usize::MAX, &[Press::End, Press::Lock][..]));
        assert_eq!(switch(true, true).1, 1, "Locked is lit");
        for (live, locked) in [(false, false), (true, false), (true, true)] {
            let (words, _, presses) = switch(live, locked);
            assert_eq!(words.len(), presses.len());
        }
    }
}
