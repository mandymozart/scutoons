//! A medium's Asset tab on the editor: opening one – from the library or a later start (the asset: its handles move the
//! asset's trim), or from a scene's `video` node (that use: its in and out points) –, Play and Stop, the pointer on its
//! timeline and on the asset's edit fields. A use's trim is written onto its use when the handle is let go (tab_view.rs
//! `trim_use`); the asset's edits are saved onto the asset when a handle or field is let go – never onto the file.
use crate::media::{self, Hit, Use};
use crate::show::Workspace;
use deck_core::trim::Trim;

impl crate::App {
    /// the medium `key` takes the front tab's stage (silent: a medium is no rig)
    pub(crate) fn media_load(&mut self, key: &str, used: Option<Use>, trim: Trim) {
        self.workspace = Workspace::Media;
        self.stage.rest();
        self.media.drag = None;
        self.media.silence();
        let api = (!self.args.api.is_empty()).then(|| crate::show::api::Api::new(self.args.api.clone()));
        self.media.open = media::Open::new(key, self.library.cdn(), api, used, trim);
        if self.media.open.is_none() {
            let (x, y) = self.cursor;
            self.refuse(format!("{key}: no medium (video, image, audio, model)"), x, y);
        }
    }

    /// Scene video node `v` opened in its Asset tab: that use, its trim on its file
    pub(crate) fn open_video_use(&mut self, v: usize) {
        let (x, y) = self.cursor;
        let Some(view) = self.scene.as_ref() else { return };
        let Some(node) = view.edit.doc.nodes.get(v) else { return };
        let Some(src) = view.edit.video_src(v) else { return self.refuse(format!("{}: no file yet", node.id), x, y) };
        let num = |k: &str| node.params.get(k).and_then(|p| p.as_float().or_else(|| p.as_integer().map(|i| i as f64))).unwrap_or(0.0);
        let trim = Trim::new(num("start"), num("end"));
        let used = Use { scene: view.edit.key(), node: node.id.clone() };
        let key = format!("video:{src}");
        // a tab of this file open without this use gives way: one tab per address
        if let Some(i) = self.tabs.find(&key) {
            self.close_tab(i);
        }
        self.adopt(&key, &crate::tabs::name_of(&key));
        self.media_load(&key, Some(used), trim);
    }

    /// Play runs the medium from its in point; Stop goes back there. A sound is heard (the preview voice); while a panic
    /// holds it stays silent.
    pub(crate) fn media_play(&mut self, on: bool) {
        let dark = self.stage.dark;
        let Some(o) = self.media.open.as_mut() else { return };
        if o.playing == on {
            return;
        }
        if on && dark && o.sort == media::Sort::Audio {
            let (x, y) = self.cursor;
            return self.refuse("a panic holds: resume first".into(), x, y);
        }
        o.playing = on;
        if !on {
            o.clock = 0.0;
        }
        self.note(if on { "Play: it runs from its in point, looped".into() } else { "Stop: back to its in point".into() });
        self.media_sound();
    }

    /// the preview voice to what the open sound does now (the cue while a show is live, else main)
    pub(crate) fn media_sound(&mut self) {
        let (x, y) = self.cursor;
        match self.media.sound(self.stage.volume(), self.stage.live) {
            Ok(Some(note)) => self.note(note),
            Ok(None) => {}
            Err(why) => {
                if let Some(o) = self.media.open.as_mut() {
                    o.playing = false;
                }
                self.refuse(why, x, y);
            }
        }
    }

    /// the clock's tempo `(bpm, per_bar)`, which beats and bars are read at
    pub(crate) fn tempo(&self) -> Option<(f64, u32)> {
        self.stage.beat.map(|(bpm, _)| (bpm, self.stage.per_bar))
    }

    /// a press on the timeline: the playhead goes there, a handle is taken; a unit's segment: its kind is read in it
    pub(crate) fn media_press(&mut self, x: f32, y: f32) -> bool {
        let Some(&(r, h)) = self.media.hits.iter().rev().find(|(r, _)| r.has(x, y)) else { return false };
        if let Hit::Unit(kind, unit) = h {
            self.change_settings(|s| s.units.set(kind, unit));
            return true;
        }
        self.media.drag = Some(h);
        self.media.field = matches!(h, Hit::Edit(_)).then_some(r);
        self.media_drag_to(x);
        true
    }

    /// the pointer held: the playhead, the point or the edit's value follows it (the use or the asset takes it when it
    /// is let go)
    pub(crate) fn media_drag_to(&mut self, x: f32) {
        if let (Some(Hit::Edit(f)), Some(r)) = (self.media.drag, self.media.field) {
            let tempo = self.tempo();
            if let Some(o) = self.media.open.as_mut() {
                let mut v = f.at(f64::from((x - r.x) / r.w.max(1.0)));
                // on the steps of the unit it is read in: whole source pixels, whole frames
                if let Some((kind, offered, first)) = f.unit() {
                    let refs = o.refs(Some(f), tempo);
                    let unit = self.settings.units.unit(kind, first, &refs);
                    if unit.whole() && offered.contains(&unit) {
                        v = unit.from_base(v, &refs).and_then(|u| unit.to_base(unit.round(u), &refs)).unwrap_or(v);
                    }
                }
                o.set_edit(f, v);
            }
            return self.media_sound();
        }
        let (Some(h), Some(r)) = (self.media.drag, self.media.line) else { return };
        let Some(o) = self.media.open.as_mut() else { return };
        let Some(length) = o.source_length() else { return };
        let (lo, hi) = if self.media.view.1 > self.media.view.0 { self.media.view } else { (0.0, length) };
        let s = lo + f64::from(((x - r.x) / r.w.max(1.0)).clamp(0.0, 1.0)) * (hi - lo);
        if h == Hit::Seek {
            o.seek(s);
            return self.media_sound();
        }
        if let Err(why) = o.set_point(h, s) {
            self.media.drag = None;
            let (x, y) = self.cursor;
            return self.refuse(why, x, y);
        }
        self.media_sound();
    }

    /// a check run's `--asset NODE[,play]`: once its scene is open, that video node in its Asset tab; `--asset play`: the
    /// medium open in front plays once it has loaded
    pub(crate) fn script_media(&mut self) {
        if let Some(o) = self.media.open.as_mut().filter(|o| o.length().is_some()) {
            if let Some(text) = self.args.asset_edits.take() {
                let detail = serde_json::json!({ "asset": { "edits": serde_json::from_str::<serde_json::Value>(&text).unwrap_or_default() } });
                o.take_asset(Ok(Some(detail)));
            }
        }
        let Some((id, play)) = self.args.asset.clone() else { return };
        if id == "play" || (id.is_empty() && play) {
            if self.media.open.as_ref().is_some_and(|o| o.length().is_some()) {
                self.args.asset = None;
                self.media_play(true);
            }
            return;
        }
        let Some(v) = self.scene.as_ref().and_then(|s| s.edit.doc.nodes.iter().position(|n| n.id == id)) else { return };
        if self.frames < 30 {
            return;
        }
        self.args.asset = None;
        self.open_video_use(v);
        if play {
            self.media_play(true);
        }
    }

    pub(crate) fn media_release(&mut self) {
        let Some(h) = self.media.drag.take() else { return };
        self.media.field = None;
        if matches!(h, Hit::Seek | Hit::Unit(..)) {
            return;
        }
        let (x, y) = self.cursor;
        let used = self.media.open.as_ref().and_then(|o| Some((o.used.clone()?, o.trim)));
        match (h, used) {
            (Hit::Start | Hit::End, Some((used, trim))) => match self.trim_use(&used, trim) {
                Ok(()) => self.note(format!("{} › {}: in {:.2} s, out {} – the file stays whole", used.scene, used.node, trim.start, if trim.end > 0.0 { format!("{:.2} s", trim.end) } else { "its end".into() })),
                Err(why) => self.refuse(why, x, y),
            },
            _ => self.save_asset_edits(),
        }
    }

    /// The open asset's edits onto the asset (`PUT /api/assets/{id}`, on a thread); its answer comes back as a note.
    pub(crate) fn save_asset_edits(&mut self) {
        let (x, y) = self.cursor;
        let Some(o) = self.media.open.as_ref() else { return };
        let Some(id) = o.asset.clone() else {
            return self.refuse(format!("{}: not in the asset registry (or not answered yet) – its edits cannot be kept", o.path), x, y);
        };
        let body = deck_space::asset_edits::to_json(&o.edits);
        let api = crate::show::api::Api::new(self.args.api.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(api.save_asset_edits(&id, &body));
        });
        self.media.saved = Some(rx);
    }

    /// a save's answer, once it is there
    pub(crate) fn media_saved(&mut self) {
        let Some(r) = self.media.saved.as_ref().and_then(|rx| rx.try_recv().ok()) else { return };
        self.media.saved = None;
        let (x, y) = self.cursor;
        match r {
            Ok(()) => self.note("the asset's edits are kept – they apply wherever it plays".into()),
            Err(why) => self.refuse(why, x, y),
        }
    }
}
