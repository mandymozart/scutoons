//! A medium's Asset tab on the editor: opening one – from the library or a later start (no use: it plays whole),
//! or from a scene's `video` node (that use, with its in and out points) –, Play and Stop, the pointer on its timeline.
//! A trim is written onto its use when the handle is let go (tab_view.rs `trim_use`), never onto the file.
use crate::media::{self, Hit, Use};
use crate::show::Workspace;
use deck_media::Trim;

impl crate::App {
    /// the medium `key` takes the front tab's stage (silent: a medium is no rig)
    pub(crate) fn media_load(&mut self, key: &str, used: Option<Use>, trim: Trim) {
        self.workspace = Workspace::Media;
        self.stage.rest();
        self.media.drag = None;
        self.media.open = media::Open::new(key, self.library.cdn(), used, trim);
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

    /// Play runs the medium from its in point; Stop goes back there
    pub(crate) fn media_play(&mut self, on: bool) {
        let Some(o) = self.media.open.as_mut() else { return };
        if o.playing == on {
            return;
        }
        o.playing = on;
        if !on {
            o.clock = 0.0;
        }
        self.note(if on { "Play: it runs from its in point, looped".into() } else { "Stop: back to its in point".into() });
    }

    /// a press on the timeline: the playhead goes there, a handle is taken
    pub(crate) fn media_press(&mut self, x: f32, y: f32) -> bool {
        let Some(&(_, h)) = self.media.hits.iter().rev().find(|(r, _)| r.has(x, y)) else { return false };
        self.media.drag = Some(h);
        self.media_drag_to(x);
        true
    }

    /// the pointer held: the playhead or the point follows it (the use takes the point when it is let go)
    pub(crate) fn media_drag_to(&mut self, x: f32) {
        let (Some(h), Some(r)) = (self.media.drag, self.media.line) else { return };
        let Some(o) = self.media.open.as_mut() else { return };
        let Some(length) = o.length() else { return };
        let (lo, hi) = if self.media.view.1 > self.media.view.0 { self.media.view } else { (0.0, length) };
        let s = lo + f64::from(((x - r.x) / r.w.max(1.0)).clamp(0.0, 1.0)) * (hi - lo);
        if h == Hit::Seek {
            return o.seek(s);
        }
        if let Err(why) = o.set_point(h, s) {
            self.media.drag = None;
            let (x, y) = self.cursor;
            self.refuse(why, x, y);
        }
    }

    /// a check run's `--asset NODE[,play]`: once its scene is open, that video node in its Asset tab
    pub(crate) fn script_media(&mut self) {
        let Some((id, play)) = self.args.asset.clone() else { return };
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
        if h == Hit::Seek {
            return;
        }
        let Some((used, trim)) = self.media.open.as_ref().and_then(|o| Some((o.used.clone()?, o.trim))) else { return };
        let (x, y) = self.cursor;
        match self.trim_use(&used, trim) {
            Ok(()) => self.note(format!("{} › {}: in {:.2} s, out {} – the file stays whole", used.scene, used.node, trim.start, if trim.end > 0.0 { format!("{:.2} s", trim.end) } else { "its end".into() })),
            Err(why) => self.refuse(why, x, y),
        }
    }
}
