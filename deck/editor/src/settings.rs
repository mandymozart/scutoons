//! The editor's settings, this machine's (the editor cache): the look and the UI size, motion & feedback, sound. The
//! main menu opens them as a floating panel. A motion switch says whether an effect may run; "reduce motion" turns
//! every one off.
use crate::tokens::{Look, Theme};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Motion {
    pub signal_flow: bool,
    pub hover_reveal: bool,
    pub focus_flash: bool,
    pub panel_transitions: bool,
    pub live_previews: bool,
    pub reduce_motion: bool,
}

impl Default for Motion {
    fn default() -> Motion {
        Motion { signal_flow: true, hover_reveal: true, focus_flash: true, panel_transitions: true, live_previews: false, reduce_motion: false }
    }
}

impl Motion {
    /// may an effect switched by `flag` run
    pub fn allows(&self, flag: bool) -> bool {
        flag && !self.reduce_motion
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sound {
    pub ui_sounds: bool,
    /// the node view's tone on focus and selection, pitched by the node's signal (deck-show focus.rs)
    pub focus_tone: bool,
    pub signal_ticks: bool,
    /// 0..1
    pub volume: f32,
}

impl Default for Sound {
    fn default() -> Sound {
        Sound { ui_sounds: true, focus_tone: deck_spores::focus::ON, signal_ticks: false, volume: 0.35 }
    }
}

impl Sound {
    /// the volume a focus tone plays at, None while switched off
    pub fn focus(&self) -> Option<f32> {
        (self.ui_sounds && self.focus_tone).then_some(self.volume)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(flatten)]
    pub look: Look,
    pub motion: Motion,
    pub sound: Sound,
    /// the Show workspace's node view: left → right or top → bottom
    pub node_flow: deck_kit::base::graph::Direction,
    /// the unit each kind of value is read in (a crop in px or %, a fade in s or frames …)
    pub units: deck_kit::editor::units::Prefs,
}

/// the editor's cache file of the settings
pub fn file(cache: &Path) -> PathBuf {
    cache.join("editor-settings.json")
}

pub fn load(file: &Path) -> Settings {
    std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(file: &Path, s: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(file, json);
    }
}

/// a switch of the panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    Colorblind,
    SignalFlow,
    HoverReveal,
    FocusFlash,
    PanelTransitions,
    LivePreviews,
    ReduceMotion,
    UiSounds,
    FocusTone,
    SignalTicks,
}

const MOTION: [(Switch, &str); 6] = [
    (Switch::SignalFlow, "Signal flow on wires"),
    (Switch::HoverReveal, "Hover reveal"),
    (Switch::FocusFlash, "Focus flash + shimmer"),
    (Switch::PanelTransitions, "Panel transitions"),
    (Switch::LivePreviews, "Live previews in nodes"),
    (Switch::ReduceMotion, "Reduce motion (all off)"),
];
const LOOK: [(Switch, &str); 1] = [(Switch::Colorblind, "Colour-blind colours")];
const SOUND: [(Switch, &str); 3] = [(Switch::UiSounds, "UI sounds"), (Switch::FocusTone, "Focus tone per signal"), (Switch::SignalTicks, "Signal ticks")];

impl Settings {
    fn flag(&mut self, s: Switch) -> &mut bool {
        match s {
            Switch::Colorblind => &mut self.look.colorblind,
            Switch::SignalFlow => &mut self.motion.signal_flow,
            Switch::HoverReveal => &mut self.motion.hover_reveal,
            Switch::FocusFlash => &mut self.motion.focus_flash,
            Switch::PanelTransitions => &mut self.motion.panel_transitions,
            Switch::LivePreviews => &mut self.motion.live_previews,
            Switch::ReduceMotion => &mut self.motion.reduce_motion,
            Switch::UiSounds => &mut self.sound.ui_sounds,
            Switch::FocusTone => &mut self.sound.focus_tone,
            Switch::SignalTicks => &mut self.sound.signal_ticks,
        }
    }

    fn is_on(&self, s: Switch) -> bool {
        *self.clone().flag(s)
    }

    /// a press on the panel, applied
    pub fn press(&mut self, p: Press) {
        match p {
            Press::Switch(s) => {
                let f = self.flag(s);
                *f = !*f;
            }
            Press::Volume(v) => self.sound.volume = v.clamp(0.0, 1.0),
            Press::Size(step) => self.look = self.look.sized(step),
            Press::Theme(theme) => self.look.theme = theme,
        }
    }
}

/// what a press on the panel does
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Press {
    Switch(Switch),
    /// the volume at this fraction
    Volume(f32),
    /// −1 smaller, +1 larger, 0 back to 100 %
    Size(i32),
    Theme(Theme),
}

/// how tall the panel's content is
pub fn height(t: &Tokens) -> f32 {
    let row = t.size.tree_row;
    t.size.row * 4.0 + row * (LOOK.len() + MOTION.len() + SOUND.len() + 1) as f32 + 2.0 * t.size.row
}

/// the panel's content in `area`; its places, and where the volume's track is (a press there sets it)
pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, s: &Settings) -> Vec<(Rect, Press)> {
    let (row, pad) = (t.size.tree_row, t.size.pad);
    let mut out = Vec::new();
    let mut y = area.y;
    y += controls::section(d, t, area.x, y, area.w, "Look", true);
    let half = (area.w - pad) / 2.0;
    for (i, (label, theme)) in [("Day", Theme::Day), ("Night", Theme::Night)].into_iter().enumerate() {
        let r = Rect { x: area.x + i as f32 * (half + pad), y, w: half, h: t.size.row - 4.0 };
        let on = s.look.theme == theme;
        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.within);
        if on {
            controls::active_tab(d, t, r, t.size.radius);
        }
        let tw = d.advance(t.size.small) * label.chars().count() as f32;
        d.text(r.x + (r.w - tw) * 0.5, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, label, if on { t.color.text } else { t.color.muted });
        out.push((r, Press::Theme(theme)));
    }
    y += t.size.row;
    // a label keeps clear of its control on the right
    let room = ((area.w - row * 1.6 - pad) / d.advance(t.size.font)).max(0.0) as usize;
    let line = |d: &mut DrawList, y: f32, label: &str| d.text(area.x, y + (row - t.size.font * 1.25) * 0.5, t.size.font, &controls::fit(label, room), t.color.text);
    let switches = |d: &mut DrawList, y: &mut f32, list: &[(Switch, &str)], out: &mut Vec<(Rect, Press)>| {
        for &(sw, label) in list {
            line(d, *y, label);
            let r = Rect { x: area.x + area.w - row * 1.6, y: *y + row * 0.15, w: row * 1.6, h: row * 0.7 };
            controls::switch(d, t, r, s.is_on(sw));
            out.push((Rect { x: area.x, y: *y, w: area.w, h: row }, Press::Switch(sw)));
            *y += row;
        }
    };
    switches(d, &mut y, &LOOK, &mut out);
    y += controls::section(d, t, area.x, y, area.w, "Motion & feedback", false);
    switches(d, &mut y, &MOTION, &mut out);
    y += controls::section(d, t, area.x, y, area.w, "Sound", false);
    switches(d, &mut y, &SOUND, &mut out);
    d.text(area.x, y + (row - t.size.font * 1.25) * 0.5, t.size.font, "Volume", t.color.text);
    let track = Rect { x: area.x + area.w * 0.5, y: y + row * 0.4, w: area.w * 0.5, h: row * 0.2 };
    d.round_rect(track.x, track.y, track.w, track.h, track.h * 0.5, t.color.within);
    d.round_rect(track.x, track.y, track.w * s.sound.volume.clamp(0.0, 1.0), track.h, track.h * 0.5, t.color.text);
    out.push((Rect { x: track.x, y, w: track.w, h: row }, Press::Volume(0.0)));
    y += row;
    y += controls::section(d, t, area.x, y, area.w, "UI size (1rem)", false);
    let h = t.size.row - 4.0;
    let third = (area.w - 2.0 * pad) / 3.0;
    let size = format!("{} px · {} %", s.look.base.round(), s.look.percent());
    let fits = ((third - pad) / d.advance(t.size.small)).max(0.0) as usize;
    let size = if size.chars().count() <= fits { size } else { format!("{} %", s.look.percent()) };
    for (i, (label, step)) in [("-", -1), (size.as_str(), 0), ("+", 1)].into_iter().enumerate() {
        let r = Rect { x: area.x + i as f32 * (third + pad), y, w: third, h };
        d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if i == 1 { t.color.control } else { t.color.within });
        let tw = d.advance(t.size.small) * label.chars().count() as f32;
        d.text(r.x + (r.w - tw) * 0.5, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, label, t.color.text);
        out.push((r, Press::Size(step)));
    }
    out
}

/// a press at `x` on a place the panel gave: the volume's fraction comes from where on its track
pub fn at(place: &Rect, p: Press, x: f32) -> Press {
    match p {
        Press::Volume(_) => Press::Volume((x - place.x) / place.w.max(1.0)),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_motion_turns_every_effect_off() {
        let mut s = Settings::default();
        assert!(s.motion.allows(s.motion.signal_flow));
        s.press(Press::Switch(Switch::ReduceMotion));
        assert!(!s.motion.allows(s.motion.signal_flow));
        assert!(!s.motion.allows(s.motion.panel_transitions));
    }

    #[test]
    fn every_switch_toggles_and_comes_back_from_the_file() {
        let mut s = Settings::default();
        for sw in LOOK.iter().chain(&MOTION).chain(&SOUND).map(|(sw, _)| *sw) {
            let was = s.is_on(sw);
            s.press(Press::Switch(sw));
            assert_ne!(s.is_on(sw), was, "{sw:?}");
        }
        s.press(Press::Volume(2.0));
        s.press(Press::Size(1));
        assert_eq!(s.sound.volume, 1.0);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
        assert_eq!(serde_json::from_str::<Settings>("{\"theme\":\"night\"}").unwrap().look.theme, crate::tokens::Theme::Night, "a part of a file keeps what it has");
    }

    #[test]
    fn the_focus_tone_is_on_by_the_cores_default_and_ui_sounds_off_silences_it() {
        let mut s = Settings::default();
        assert_eq!(s.sound.focus_tone, deck_spores::focus::ON);
        assert_eq!(s.sound.focus(), Some(s.sound.volume));
        s.press(Press::Switch(Switch::FocusTone));
        assert_eq!(s.sound.focus(), None);
        s.press(Press::Switch(Switch::FocusTone));
        s.press(Press::Switch(Switch::UiSounds));
        assert_eq!(s.sound.focus(), None, "UI sounds off: every UI sound");
        assert_eq!(serde_json::from_str::<Settings>("{\"sound\":{\"ui_sounds\":true}}").unwrap().sound.focus_tone, deck_spores::focus::ON, "an older file gets the default");
    }

    #[test]
    fn the_panel_places_every_switch_the_volume_and_the_size() {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        let s = Settings::default();
        let places = draw(&mut d, &t, Rect { x: 0.0, y: 0.0, w: 320.0, h: 600.0 }, &s);
        assert_eq!(places.len(), 2 + LOOK.len() + MOTION.len() + SOUND.len() + 1 + 3);
        let (_, day) = places[0];
        let mut pressed = s;
        pressed.press(day);
        assert_eq!((pressed.look.theme, places[1].1), (Theme::Day, Press::Theme(Theme::Night)), "the look first: day and night");
        let last = places.last().unwrap().0;
        assert!(last.y + last.h <= height(&t) + 0.01, "the content fits its height");
        let (track, p) = places.iter().find(|(_, p)| matches!(p, Press::Volume(_))).copied().unwrap();
        assert_eq!(at(&track, p, track.x + track.w * 0.5), Press::Volume(0.5));
    }
}
