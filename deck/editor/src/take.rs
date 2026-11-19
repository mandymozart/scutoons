//! Build's DOP panel records a take (LIVE-CAMERAS.md §7): the phone holding the selected camera of the open scene is
//! recorded – Record in the panel, or the phone's own Record (`hand.record` from the camera's holder) – and kept as a move
//! of the scene's draft when it stops. The recording, its keys and the move are the core's (deck-space space/dop take.rs,
//! scene_dop.rs `add_take`); here the phones' words are handed in and the stage looks through the take while it runs.
use deck_space::live_cameras::{camera_id, camera_of};
use deck_space::space::dop::take::Recorder;
use deck_space::space::dop::View;
use protocol::Message;

/// what the panel shows of a take being recorded: the camera, how long so far (s), the poses kept
#[derive(Debug, Clone, PartialEq)]
pub struct Rec {
    pub camera: String,
    pub secs: f32,
    pub poses: usize,
}

impl crate::App {
    /// the live camera the DOP panel has selected: the picked shot's, else the scene's primary camera
    fn take_camera(&self) -> Option<String> {
        let view = self.scene.as_ref()?;
        let shot = view.dop.shot.and_then(|i| view.edit.camera().shots.get(i).map(|s| s.id.clone()));
        Some(camera_id(&view.edit.key(), shot.as_deref()))
    }

    /// Record in the DOP panel: the selected camera's phone is recorded; pressed while recording, the take is kept
    pub(crate) fn take_toggle(&mut self, x: f32, y: f32) {
        if self.take.is_some() {
            return self.take_keep(x, y);
        }
        let Some(camera) = self.take_camera() else { return };
        if self.show.holder_of(&camera).is_none() {
            return self.refuse(format!("no phone holds {camera} – its QR: Perform › Session"), x, y);
        }
        self.take_begin(camera);
    }

    /// the take of `camera`'s phone begins on that camera's view: its shot's, else where the scene opens, else the stage's
    pub(crate) fn take_begin(&mut self, camera: String) {
        let dop = &self.stage.space.dop;
        let anchor = match camera_of(&camera).1 {
            Some(shot) => dop.view(shot),
            None => dop.camera.opening().map(View::of_shot),
        };
        let anchor = anchor.unwrap_or_else(|| View::of(&self.stage.space.camera));
        self.note(format!("recording {camera}: the phone's moves become a take"));
        self.take = Some(Recorder::new(&camera, anchor));
    }

    /// Stop: the take kept as a move of the scene (in beats while the tempo clock runs), or why not
    pub(crate) fn take_keep(&mut self, x: f32, y: f32) {
        let Some(rec) = self.take.take() else { return };
        let bpm = self.stage.space.dop.clock.filter(|c| c.running).map(|c| c.bpm);
        let Some(view) = self.scene.as_mut() else { return };
        view.dop.rec = None;
        let Some((take, end)) = rec.take(bpm) else { return self.refuse("the take is too short to be a move: no poses came".into(), x, y) };
        let keys = take.keys.len();
        match view.edit.add_take(take, end.shot("")) {
            Ok(i) => {
                view.dop.kept = Some(i);
                let id = view.edit.camera().moves[i].id.clone();
                let place = view.edit.key();
                self.dop_done(Ok(()), format!("kept as {place} > move:{id} – {} s, {keys} keys", crate::number(rec.secs())), x, y);
            }
            Err(e) => self.refuse(e, x, y),
        }
    }

    /// Discard: the take is dropped
    pub(crate) fn take_discard(&mut self) {
        if self.take.take().is_some() {
            if let Some(view) = self.scene.as_mut() {
                view.dop.rec = None;
            }
            self.note("the take was dropped".into());
        }
    }

    /// Each frame: the phones' pose, Record and Recenter heard to the take; the stage looks through it while it runs.
    pub(crate) fn take_frame(&mut self) {
        let hands = std::mem::take(&mut self.show.hands);
        let (x, y) = self.cursor;
        for e in hands {
            match &e.message {
                // the phone's own Record: a camera of the open scene it holds; its Stop keeps the take
                Message::HandRecord(r) if r.on && self.take.is_none() => {
                    let scene = self.scene.as_ref().map(|v| v.edit.key());
                    if let Some(camera) = self.show.held_by(&e.from).filter(|c| Some(camera_of(c).0) == scene.as_deref()) {
                        self.take_begin(camera);
                    }
                }
                Message::HandRecord(r) if !r.on && self.take_holder() == Some(e.from.clone()) => self.take_keep(x, y),
                Message::HandPose(p) if self.take_holder() == Some(e.from.clone()) => {
                    if let Some(rec) = self.take.as_mut() {
                        rec.pose(p.t, &p.p, &p.q);
                    }
                }
                Message::HandRecenter(_) if self.take_holder() == Some(e.from.clone()) => {
                    if let Some(rec) = self.take.as_mut() {
                        rec.recenter();
                    }
                }
                _ => {}
            }
        }
        // the phone let go while recording: what it did is kept
        if self.show.relay().is_some() && self.take_holder().is_none() && self.take.is_some() {
            self.take_keep(x, y);
        }
        let Some(rec) = self.take.as_ref() else { return };
        let shown = Rec { camera: rec.camera().to_string(), secs: rec.secs(), poses: rec.poses() };
        if let Some(v) = rec.view() {
            v.apply(&mut self.stage.space.camera);
        }
        if let Some(view) = self.scene.as_mut() {
            view.dop.rec = Some(shown);
        }
    }

    /// the client holding the camera being recorded
    fn take_holder(&self) -> Option<String> {
        self.take.as_ref().and_then(|r| self.show.holder_of(r.camera()))
    }
}
