//! Edit and Play of an item in the editor (decisions/deck.md [deck-editor], provisional: the owner tests it). Edit is always
//! live: the item's clock, far events, values and MIDI drive it. Only the person's own hand is saved; a value from outside
//! is a live layer over the hand's, shown beside it and never saved, dropped on Stop or when Edit is left. Play runs the
//! item as a deck from the start and Stop returns to Edit. The mixer's solos and the armed track hold for the session.
use deck_core::solo;
use std::collections::BTreeSet;

/// One control input the live layer holds: the hand's value under it (what is saved) and the live one (what plays).
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    /// the compiled slot, `<node>.<port>` (`drone_osc.freq`)
    pub slot: String,
    pub hand: f32,
    pub live: f32,
}

/// What outside inputs set on the item's control inputs, over the hand's values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LiveLayer {
    held: Vec<Held>,
}

impl LiveLayer {
    /// A value from outside on `slot`, whose value is `now`: `now` is the hand's when the layer does not hold it yet.
    pub fn outside(&mut self, slot: &str, now: f32, value: f32) {
        match self.held.iter_mut().find(|h| h.slot == slot) {
            Some(h) => h.live = value,
            None => self.held.push(Held { slot: slot.to_string(), hand: now, live: value }),
        }
    }

    /// The hand took `slot`: what it sets is the hand's again, and is saved. True when the layer held it.
    pub fn hand(&mut self, slot: &str) -> bool {
        let before = self.held.len();
        self.held.retain(|h| h.slot != slot);
        self.held.len() != before
    }

    pub fn held(&self, slot: &str) -> Option<&Held> {
        self.held.iter().find(|h| h.slot == slot)
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// what `slot` saves when the engine has `engine` on it: the hand's value, never the live one
    pub fn saved(&self, slot: &str, engine: f32) -> f32 {
        self.held(slot).map_or(engine, |h| h.hand)
    }

    /// The layer dropped: every slot it held, back to the hand's value.
    pub fn drop_all(&mut self) -> Vec<(String, f32)> {
        std::mem::take(&mut self.held).into_iter().map(|h| (h.slot, h.hand)).collect()
    }
}

/// The mixer's solos and the armed track: held for the session, never saved with the item.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Monitor {
    solo: BTreeSet<String>,
    armed: Option<String>,
}

impl Monitor {
    /// `track` soloed, or not any more; whether it is now
    pub fn solo(&mut self, track: &str) -> bool {
        if !self.solo.remove(track) {
            self.solo.insert(track.to_string());
        }
        self.soloed(track)
    }

    pub fn soloed(&self, track: &str) -> bool {
        self.solo.contains(track)
    }

    /// `track` armed (the one that takes the hand: keys, a MIDI keyboard, knobs), or disarmed; one at a time
    pub fn arm(&mut self, track: &str) -> bool {
        if self.armed.as_deref() == Some(track) {
            self.armed = None;
        } else {
            self.armed = Some(track.to_string());
        }
        self.is_armed(track)
    }

    pub fn armed(&self) -> Option<&str> {
        self.armed.as_deref()
    }

    pub fn is_armed(&self, track: &str) -> bool {
        self.armed.as_deref() == Some(track)
    }

    /// Whether `track` is heard: the engine's one solo rule (deck-core `solo`) over the tracks' audio sends (from, to);
    /// arming turns on a track's monitoring, so the armed one is heard through another's solo.
    pub fn heard(&self, track: &str, sends: &[(String, String)]) -> bool {
        let soloed = |list: Vec<String>| list.iter().any(|t| self.soloed(t));
        let track = track.to_string();
        let first: Vec<String> = sends.iter().filter(|(from, _)| *from == track).map(|(_, to)| to.clone()).collect();
        let (up, down) = (solo::upstream(sends, &track), solo::downstream(sends, &first, &track));
        solo::heard(!self.solo.is_empty(), self.soloed(&track), soloed(up), soloed(down)) || self.is_armed(&track)
    }

    /// the solos and the arm of tracks the item no longer has are forgotten
    pub fn keep_only(&mut self, tracks: &[&str]) {
        self.solo.retain(|t| tracks.contains(&t.as_str()));
        if self.armed.as_deref().is_some_and(|a| !tracks.contains(&a)) {
            self.armed = None;
        }
    }
}

/// where the item in front sounds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outlet {
    /// the deck's outputs
    Main,
    /// the cue (headphones), folded beside what the device's outputs play
    Cue,
}

/// While a show is live the stage is a preview on the cue; Edit plays to the cue when the device has one, else to main;
/// Play to the deck's outputs.
pub fn outlet(playing: bool, live: bool, cue: bool) -> Outlet {
    if live || (!playing && cue) {
        Outlet::Cue
    } else {
        Outlet::Main
    }
}

/// What a switch between Edit and Play does to what plays.
#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    /// the live layer dropped: these slots back to the hand's values
    pub restore: Vec<(String, f32)>,
    /// Play: built again from the start – states reset, the timeline at 0
    pub from_start: bool,
}

/// Play: the live layer goes, the item runs as a deck from the start.
pub fn play(layer: &mut LiveLayer) -> Switch {
    Switch { restore: layer.drop_all(), from_start: true }
}

/// Stop, or Edit left: the live layer goes; what Play changed is the editor's to drop (back to where Edit was).
pub fn stop(layer: &mut LiveLayer) -> Switch {
    Switch { restore: layer.drop_all(), from_start: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_outside_value_drives_but_the_hand_is_what_is_saved() {
        let mut l = LiveLayer::default();
        l.outside("drone_osc.freq", 220.0, 440.0);
        l.outside("drone_osc.freq", 440.0, 330.0);
        let h = l.held("drone_osc.freq").expect("held");
        assert_eq!((h.hand, h.live), (220.0, 330.0), "the hand's value stays under the live one");
        assert_eq!(l.saved("drone_osc.freq", 330.0), 220.0, "the live value is never saved");
        assert_eq!(l.saved("drone_osc.gain", 0.5), 0.5, "a slot the layer does not hold saves what plays");
    }

    #[test]
    fn a_hand_turn_takes_the_slot_back_and_is_saved() {
        let mut l = LiveLayer::default();
        l.outside("drone_osc.freq", 220.0, 440.0);
        assert!(l.hand("drone_osc.freq"));
        assert_eq!(l.saved("drone_osc.freq", 500.0), 500.0, "the hand's turn is saved");
        assert!(!l.hand("drone_osc.freq") && l.is_empty());
    }

    #[test]
    fn play_starts_from_the_start_and_stop_drops_the_live_layer() {
        let mut l = LiveLayer::default();
        l.outside("a.x", 0.1, 0.9);
        let p = play(&mut l);
        assert_eq!(p, Switch { restore: vec![("a.x".into(), 0.1)], from_start: true });
        assert!(l.is_empty());
        l.outside("a.x", 0.1, 0.7);
        let s = stop(&mut l);
        assert_eq!(s, Switch { restore: vec![("a.x".into(), 0.1)], from_start: false });
        assert!(stop(&mut l).restore.is_empty(), "nothing left to drop");
    }

    #[test]
    fn a_solo_silences_the_others_but_not_the_armed_track() {
        let mut m = Monitor::default();
        let heard = |m: &Monitor, t: &str| m.heard(t, &[]);
        assert!(heard(&m, "drums") && heard(&m, "bass"));
        assert!(m.solo("drums"));
        assert!(heard(&m, "drums") && !heard(&m, "bass") && !heard(&m, "keys"));
        assert!(m.arm("keys"));
        assert!(heard(&m, "keys"), "arming monitors it");
        assert!(m.arm("bass") && !m.is_armed("keys"), "one armed at a time");
        assert!(!m.arm("bass") && m.armed().is_none());
        assert!(!m.solo("drums") && heard(&m, "bass"), "the solo off: all heard");
        m.solo("gone");
        m.arm("gone");
        m.keep_only(&["drums"]);
        assert!(!m.soloed("gone") && m.armed().is_none());
    }

    #[test]
    fn a_solo_keeps_the_send_and_the_folder_it_feeds() {
        // keys send into reverb; drums play into the kit folder; bass stands alone
        let sends: Vec<(String, String)> = [("keys", "reverb"), ("drums", "kit")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let mut m = Monitor::default();
        m.solo("keys");
        assert!(m.heard("keys", &sends) && m.heard("reverb", &sends), "its send stays");
        assert!(!m.heard("drums", &sends) && !m.heard("kit", &sends) && !m.heard("bass", &sends));
        m.solo("keys");
        m.solo("kit");
        assert!(m.heard("kit", &sends) && m.heard("drums", &sends), "a folder keeps its children");
        assert!(!m.heard("keys", &sends) && !m.heard("reverb", &sends) && !m.heard("bass", &sends));
        m.arm("bass");
        assert!(m.heard("bass", &sends), "the armed track is heard through any solo");
    }

    #[test]
    fn edit_plays_to_the_cue_when_there_is_one() {
        assert_eq!(outlet(false, false, true), Outlet::Cue);
        assert_eq!(outlet(false, false, false), Outlet::Main);
        assert_eq!(outlet(true, false, true), Outlet::Main, "Play: the deck's outputs");
        assert_eq!(outlet(true, true, false), Outlet::Cue, "a live show: a preview on the cue");
    }
}
