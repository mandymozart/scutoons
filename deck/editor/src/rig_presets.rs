//! The rig's presets in Build (decisions/deck.md [deck-editor], provisional; the rules are deck-space rig_preset.rs): the
//! inspector's PRESET line, a preset loaded into the working state (which saves itself as the rig's draft), the working
//! state saved as a new preset or over the loaded one (a factory one derives), Play's Keep, and a scene member's preset.
use crate::inspector::PresetLine;
use deck_space::rig::rig_preset::{self, RigPreset, Saved, Values};
use std::time::Instant;

impl crate::App {
    /// the rig's knobs as your hand left them, by their addresses (the live layer is not yours)
    fn preset_hand(&self) -> Values {
        let slots = self.stage.knobs.iter().map(|k| (k.path.clone(), self.stage.saved(k.node, &k.port)));
        rig_preset::values_at(&self.stage.bundle.rig, slots)
    }

    fn preset_taken(&self) -> Vec<String> {
        self.rigs.preset_ids()
    }

    /// the PRESET line of the rig's inspector; none shown while a scene is open (a member has its own)
    pub(crate) fn preset_line(&self) -> PresetLine {
        if self.scene.is_some() {
            return PresetLine::default();
        }
        let presets = self.rigs.presets(&self.stage.bundle.rig.subject);
        let loaded = self.rigs.preset.as_ref().filter(|p| p.rig == self.stage.bundle.rig.subject);
        PresetLine {
            shown: true,
            names: presets.iter().map(|p| p.name.clone()).collect(),
            loaded: loaded.and_then(|l| presets.iter().position(|p| p.id == l.id)),
            dirty: loaded.is_some_and(|l| l.dirty(&self.preset_hand())),
            factory: loaded.is_some_and(|l| l.factory),
        }
    }

    /// The rig's preset `i` into its working state: its knobs where the preset puts them (an undo step; in Edit it saves
    /// itself as the rig's draft, in Play it is dropped on Stop).
    pub(crate) fn preset_load(&mut self, i: usize) {
        let (x, y) = self.cursor;
        let Some(p) = self.rigs.presets(&self.stage.bundle.rig.subject).into_iter().nth(i) else { return };
        let (next, missing) = p.load(&self.stage.bundle);
        let before = self.stage.bundle.clone();
        if self.take(next, x, y) {
            self.undo.push(before);
        }
        let left_out = if missing.is_empty() { String::new() } else { format!(" – {} knob(s) the rig has not got left out", missing.len()) };
        self.note(format!("preset {} loaded into the working state{left_out}", p.name));
        self.rigs.preset = Some(p);
    }

    /// the working state saved as a new preset named `name`
    pub(crate) fn preset_save_as(&mut self, name: &str) {
        let (x, y) = self.cursor;
        if name.trim().is_empty() {
            return self.refuse("a preset has a name".into(), x, y);
        }
        let taken = self.preset_taken();
        let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
        let p = RigPreset::of_values(&self.stage.bundle.rig.subject, name, self.preset_hand(), &taken);
        self.preset_send(p, String::new());
    }

    /// the loaded preset saved with the working state: your own overwritten, a factory one stays and a derived one is yours
    pub(crate) fn preset_save(&mut self) {
        let Some(loaded) = self.rigs.preset.clone() else { return };
        let taken = self.preset_taken();
        let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
        match loaded.save(&self.preset_hand(), &taken) {
            Saved::Over(p) => self.preset_send(p, String::new()),
            Saved::Derived(p) => self.preset_send(p, format!(" – the factory preset {} stays, this one names it as its origin", loaded.name)),
        }
    }

    fn preset_send(&mut self, p: RigPreset, why: String) {
        let name = p.name.clone();
        let said = self.rigs.save_preset(p).unwrap_or_else(|| format!("preset {name} saving …"));
        self.note(format!("{said}{why}"));
    }

    /// Play's Keep: what Play changed is the working state now – Stop keeps it, and it saves itself (the live layer is not
    /// kept: only your hand)
    pub(crate) fn keep_play(&mut self) {
        let hand: Vec<(String, f32)> = self.stage.knobs.iter().map(|k| (k.path.clone(), self.stage.saved(k.node, &k.port))).collect();
        let (bundle, undo, scene) = (self.stage.bundle.clone(), self.undo.len(), self.scene.as_ref().map(|v| v.edit.doc.clone()));
        let Some(left) = self.play.left.as_mut() else { return };
        (left.bundle, left.knobs, left.undo, left.scene) = (bundle, hand, undo, scene);
        self.play.touched = false;
        if self.scene.is_none() {
            let (x, y) = self.cursor;
            self.keep_copy(x, y);
            self.rigs.auto.edit(Instant::now());
        }
        self.note("Keep: what Play changed is the working state now – Stop keeps it, it saves itself".into());
    }

    /// The selected scene member plays its rig on its preset `i` (`usize::MAX`: on the rig's own values); the scene's
    /// values stay single knobs over it.
    pub(crate) fn scene_preset(&mut self, i: usize) {
        let (x, y) = self.cursor;
        let Some(view) = self.scene.as_mut() else { return };
        let Some(m) = view.member() else { return };
        let id = match i {
            usize::MAX => None,
            i => match view.edit.presets(m).get(i) {
                Some(p) => Some(p.id.clone()),
                None => return,
            },
        };
        let said = match view.edit.set_preset(m, id.as_deref()).and_then(|said| view.refresh().map(|_| said)) {
            Ok(said) => said,
            Err(why) => return self.refuse(why, x, y),
        };
        self.replay_scene();
        self.note(said);
    }

    /// A check run's `--rig-preset`: `save-as:NAME` saves the working state as a preset at frame 60; `dirty:NAME` also
    /// turns the first knob at frame 66; `member:NAME` gives the `--member` holder's rig a preset (its values halved) and
    /// puts the member on it at frame 77.
    pub(crate) fn script_presets(&mut self) {
        let Some(spec) = self.args.rig_preset.clone() else { return };
        let (what, name) = spec.split_once(':').unwrap_or((spec.as_str(), "soft"));
        match (what, self.frames) {
            ("save-as" | "dirty", 60) => self.preset_save_as(name),
            ("dirty", 66) => {
                let Some(k) = self.stage.knobs.first() else { return };
                let (node, port, name, min, max) = (k.node, k.port_index, k.port.clone(), k.min, k.max);
                self.stage.set(node, port, min + (max - min) * 0.85, min, max);
                self.persist(node, &name);
            }
            ("member", 77) => {
                let Some(view) = self.scene.as_ref() else { return };
                let Some(b) = view.member().and_then(|m| view.edit.members.get(m)?.bundle.clone()) else { return };
                let Ok(mut p) = RigPreset::capture(&b, name, &[]) else { return };
                p.values.values_mut().for_each(|v| *v *= 0.5);
                self.rigs.save_preset(p);
                let all = self.rigs.all_presets();
                if let Some(v) = self.scene.as_mut() {
                    v.presets = all;
                    let _ = v.refresh();
                }
                self.scene_preset(0);
            }
            _ => {}
        }
    }
}
