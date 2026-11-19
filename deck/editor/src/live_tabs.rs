//! One live show in the editor (decisions/deck.md 2026-11-18 [workspaces]): Go live on a Show tab joins its show to the
//! relay as its conductor; while another show is live (a tab behind holds it), the editor asks first and ends that one.
//! A live show behind keeps running – its conductor hears the relay and keeps its clock every frame. End leaves the relay,
//! Lock refuses the document's edits (show/live.rs).
//!
//! While a show is live it owns the shared things (#630): the stage's clock is the show's (locked: no key moves it), and
//! what the stage plays – a Scene or Asset tab, Edit or Play – is a preview folded onto the cue, never on the outlets and
//! with no MIDI clock out; a pre-listened cell is a second engine on the same cue; the cue mix blends the two sides.
use crate::show::cue::Ask;
use crate::tab_view::Kept;

/// a press on a Show tab's live switch in the toolbar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Go,
    End,
    Lock,
}

/// what the status bar says of the cue: nothing heard on a device without a cue of its own (meters only), else what is on
/// it; None = nothing to say
pub fn cue_note(live: bool, heard: Option<bool>, prelistening: Option<&str>) -> Option<(String, bool)> {
    match (heard, prelistening) {
        (Some(false), Some(_)) => Some(("no cue group: the pre-listen is silent, meters only – map `cue` in Audio".into(), true)),
        (Some(false), None) if live => Some(("no cue group: the preview is silent, meters only – map `cue` in Audio".into(), true)),
        (_, Some(p)) => Some((format!("on the cue: {p}"), false)),
        _ if live => Some(("live: previews on the cue".into(), false)),
        _ => None,
    }
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

    /// the live show: the one in front, else one behind
    fn live_host(&self) -> Option<&crate::show::Host> {
        if self.show.live() {
            return Some(&self.show);
        }
        self.tabs.list.iter().find_map(|t| match &t.kept {
            Some(Kept::Show(h)) if h.live() => Some(&**h),
            _ => None,
        })
    }

    /// Each frame (#630): the stage follows the live show – a preview on the cue while one is live, its clock held to the
    /// show's, the cue mix the live show's; a pre-listen asked in front goes onto the cue; the Show tab hears back what
    /// is there.
    pub(crate) fn follow_cue(&mut self) {
        let host = self.live_host();
        let live = host.is_some();
        let held = host.and_then(|h| h.sub.clock_at(crate::show::now())).map(|c| deck_core::tempo::Held { bpm: c.bpm, beat: c.beat, per_bar: c.per_bar, running: c.running, at_ns: deck_native::clock::now_ns() });
        let mix = host.map(|h| h.cue_mix);
        if let Err(e) = self.stage.set_live(live) {
            crate::log::post(deck_ui::messages::Level::Warn, "cue", format!("{e:#}"));
        }
        self.stage.hold(held);
        if let Some(m) = mix {
            self.stage.set_cue_mix(m);
        }
        if self.media.sound_stale(live) {
            self.media_sound();
        }
        match self.show.to_prelisten.take() {
            Some(Ask::Play { label, scene, set }) => {
                if let Err(e) = self.stage.prelisten(&label, &scene, &set) {
                    self.show.refuse(format!("not pre-listened: {e:#}"));
                }
            }
            Some(Ask::Stop) => {
                if let Err(e) = self.stage.stop_prelisten() {
                    self.show.refuse(format!("{e:#}"));
                }
            }
            None => {}
        }
        self.show.prelistening = self.stage.prelisten.as_ref().map(|p| p.label.clone());
        self.show.cue_note = cue_note(live, self.stage.cue_heard(), None).filter(|n| n.1).map(|n| n.0);
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
    fn the_cue_note_says_why_nothing_is_heard() {
        assert_eq!(cue_note(false, Some(true), None), None);
        assert_eq!(cue_note(true, Some(true), None).unwrap(), ("live: previews on the cue".to_string(), false));
        assert!(cue_note(true, Some(false), None).unwrap().1, "live without a cue: a warning");
        assert!(cue_note(false, Some(false), Some("character:keys · left")).unwrap().0.contains("pre-listen is silent"));
        assert_eq!(cue_note(true, Some(true), Some("scene:nave · left")).unwrap().0, "on the cue: scene:nave · left");
        assert_eq!(cue_note(false, Some(false), None), None, "nothing live, nothing on the cue: quiet");
    }

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
