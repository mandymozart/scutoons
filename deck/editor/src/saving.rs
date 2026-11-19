//! The editor saves itself (autosave.rs): each frame the rig or the scene open in Build, the module and Show's documents
//! send what a finished burst of edits changed; leaving a workspace, a view or the editor sends it at once, ctrl+s too.
//! The toolbar's context pill says where the open one stands.
use crate::autosave::{fingerprint, State};
use crate::show::{self, api::Refusal};
use std::time::{Duration, Instant};

/// how long quitting waits for the saves on their way
const QUIT_WAIT: Duration = Duration::from_secs(5);

impl crate::App {
    /// each frame: the answers, then what is due (all of it when the workspace changed)
    pub(crate) fn autosave(&mut self) {
        let now = Instant::now();
        let left = self.saved_in != self.workspace;
        self.saved_in = self.workspace;
        if !self.autosaves {
            return;
        }
        self.save_scene(left, now);
        self.save_rig(left, now);
        self.module.autosave(left, now);
        self.show.autosave(left, now);
    }

    /// leaving the rig or the scene open in Build for another: what it edited goes now
    pub(crate) fn leave_view(&mut self) {
        let now = Instant::now();
        if self.autosaves {
            self.save_rig(true, now);
            self.save_scene(true, now);
        }
    }

    /// ctrl+s: what the workspace edits goes now
    pub(crate) fn save_now(&mut self) {
        let now = Instant::now();
        if self.playing() {
            return self.note("Play: nothing to save – what you change now is dropped on Stop".to_string());
        }
        match self.workspace {
            show::Workspace::Stage if self.scene.is_some() => {
                if self.scene.as_ref().is_some_and(|v| v.auto.now()) {
                    self.scene_save(now);
                }
            }
            show::Workspace::Stage => {
                if self.rigs.saves() && self.rigs.auto.now() {
                    self.rigs.save(&self.stage.base, &self.stage.bundle, now);
                }
            }
            show::Workspace::Module => self.module.save_now(now),
            // a medium saves nothing: its trim is its use's, saved with that scene
            show::Workspace::Media => {}
            _ => self.show.save_now(now),
        }
    }

    fn save_rig(&mut self, leave: bool, now: Instant) {
        let r = &self.rigs;
        if self.scene.is_none() && r.saves() && (r.auto.due(now) || (leave && r.auto.leave())) {
            self.rigs.save(&self.stage.base, &self.stage.bundle, now);
        }
    }

    fn save_scene(&mut self, leave: bool, now: Instant) {
        let (x, y) = self.cursor;
        let Some(v) = self.scene.as_mut() else { return };
        if let Some((doc, answer)) = v.auto.poll() {
            match answer {
                Ok(version) => {
                    if v.edit.doc == doc {
                        v.edit.saved_as(version);
                    } else {
                        v.edit.base = Some(version);
                    }
                    self.show.scene_saved(&doc, version);
                }
                Err(Refusal::Offline(_)) => {}
                Err(e) => self.refuse(e.to_string(), x, y),
            }
        }
        // Play's changes are dropped on Stop: never saved
        if self.playing() {
            return;
        }
        let Some(v) = self.scene.as_mut() else { return };
        // a member open as its rig: its knobs go into the scene on the way up, or on leaving
        if v.inside.is_none() {
            v.auto.look(fingerprint(&v.edit.doc), now);
        }
        if (v.inside.is_none() && v.auto.due(now)) || (leave && v.auto.leave()) {
            self.scene_save(now);
        }
    }

    /// the toolbar pill's words for what the workspace edits; none where nothing saves itself
    pub(crate) fn save_pill(&self) -> Option<String> {
        match self.workspace {
            show::Workspace::Stage => match &self.scene {
                Some(v) => Some(v.auto.pill()),
                None => self.rigs.saves().then(|| self.rigs.auto.pill()),
            },
            show::Workspace::Module => self.module.open.as_ref().map(|_| self.module.auto.pill()),
            show::Workspace::Show => Some(self.show.save_pill()),
            show::Workspace::Home | show::Workspace::Media => None,
        }
    }

    /// a check run's `--save-state`: Show's pill in that state, on v828 (the screenshots of its states)
    pub(crate) fn script_save_state(&mut self) {
        let Some(s) = self.args.save_state.clone() else { return };
        let id = self.show.shown().id.clone();
        let a = self.show.saves.entry(id).or_default();
        a.version = Some(828);
        a.state = match s.as_str() {
            "saving" => State::Saving,
            "offline" => State::Offline,
            "conflict" => State::Conflict(Some(830)),
            _ => State::Clean,
        };
    }

    /// quitting: what is edited goes now, and the answers are waited for a moment
    pub(crate) fn flush_all(&mut self) {
        self.stop_play();
        let now = Instant::now();
        self.leave_view();
        if self.autosaves {
            self.module.autosave(true, now);
            self.show.autosave(true, now);
        }
        let until = now + QUIT_WAIT;
        self.rigs.auto.wait(until);
        if let Some(v) = self.scene.as_mut() {
            v.auto.wait(until);
        }
        self.module.auto.wait(until);
        self.show.wait_saves(until);
        if self.autosaves {
            self.wait_behind(until);
        }
        if [&self.rigs.auto.state, &self.module.auto.state].contains(&&State::Offline) {
            tracing::warn!("quit offline: the local copies keep what was not sent");
        }
    }
}
