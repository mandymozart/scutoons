//! The tabs on the editor (tabs.rs): what a tab keeps while it is behind, how the front one goes behind and another
//! comes forward, opening an item (its tab when it is open, else a new tab in front), closing, Home, and what a later
//! start reopens. Only the front tab runs: the one stage plays it; a tab behind is its document and its view (camera,
//! selection, level, Play or Wire), frozen. The panels are the view's (panels.rs), not each tab's.
use crate::autosave::Autosave;
use crate::show::{self, Workspace};
use crate::tabs::{self, Kind, View};
use crate::{bar, home, inspector, module, rigs};
use deck_space::rig::Bundle;

type Camera = deck_space::space::camera::Orbit;

/// what a tab behind keeps
pub enum Kept {
    Rig(Box<RigKept>),
    Scene(Box<SceneKept>),
    Module(Box<ModuleKept>),
    /// a show behind another show (the one in front, or the last one in front, is the editor's own host)
    Show(Box<show::Host>),
    /// a medium: its file as loaded, its use and trim, where its playhead stood
    Media(Box<crate::media::Open>),
}

pub struct RigKept {
    bundle: Bundle,
    base: Bundle,
    subject: String,
    auto: Autosave<(Vec<(String, u64)>, Bundle)>,
    draft: rigs::Copy,
    undo: Vec<Bundle>,
    sel: inspector::Sel,
    path: Vec<String>,
    wire: bool,
    camera: Camera,
}

pub struct SceneKept {
    view: crate::scene::SceneView,
    camera: Camera,
    wire: bool,
}

pub struct ModuleKept {
    open: Option<module::Open>,
    descriptor: Option<deck_catalog::Descriptor>,
    problems: Vec<module::problems::Row>,
    auto: Autosave<(String, serde_json::Value)>,
}

/// a rig's subject as a tab's address and name: `character.catpope` → `character:catpope`, catpope
pub fn rig_key(subject: &str) -> (String, String) {
    match subject.split_once('.') {
        Some((kind, id)) => (format!("{kind}:{id}"), id.to_string()),
        None => (format!("character:{subject}"), subject.to_string()),
    }
}

/// a tab's address as a rig's subject: `character:catpope` → `character.catpope`
pub fn subject_of(key: &str) -> String {
    key.replacen(':', ".", 1)
}

/// the empty module tab's address: a module is picked or made in it
pub const NEW_MODULE: &str = "module:";
/// a new show's tab before it has a name
const EMPTY_SHOW: &str = "show:";

impl crate::App {
    /// what the editor has in front now, as a tab's address and name (none: Home)
    pub(crate) fn live_key(&self) -> Option<(String, String)> {
        match self.workspace {
            Workspace::Home => None,
            Workspace::Stage => Some(match &self.scene {
                Some(v) => (v.edit.key(), v.edit.doc.id.clone()),
                None => rig_key(&self.stage.bundle.rig.subject),
            }),
            Workspace::Module => Some(self.module.open.as_ref().map_or_else(|| (NEW_MODULE.to_string(), "new module".to_string()), |o| (o.key(), o.id.clone()))),
            Workspace::Media => self.media.open.as_ref().map(|o| (o.key.clone(), tabs::name_of(&o.key))),
            Workspace::Show => {
                let id = self.show.sub.conductor().show().id.clone();
                Some((format!("show:{id}"), id))
            }
        }
    }

    /// whether what is in front has changes the server has not got
    pub(crate) fn live_dirty(&self) -> bool {
        let dirty = |pending: bool, state: &crate::autosave::State| pending || *state != crate::autosave::State::Clean;
        match self.workspace {
            Workspace::Home | Workspace::Media => false,
            Workspace::Stage => match &self.scene {
                Some(v) => dirty(v.auto.pending(), &v.auto.state),
                None => dirty(self.rigs.auto.pending(), &self.rigs.auto.state),
            },
            Workspace::Module => dirty(self.module.auto.pending(), &self.module.auto.state),
            Workspace::Show => show_dirty(&self.show),
        }
    }

    /// the front tab goes behind: what it edited is sent, its state kept (a show stays the editor's host); what the
    /// stage holds that is not the tab's own (its item still loading) is not kept, the tab opens from its address again
    fn freeze(&mut self) -> Option<Kept> {
        // switching away stops its Play
        self.stop_play();
        self.leave_view();
        self.menu = None;
        self.edit = None;
        let own = self.tabs.front().map(|t| t.key.clone()) == self.live_key().map(|k| k.0);
        if !own && self.workspace == Workspace::Stage {
            self.drop_scene();
            return None;
        }
        let camera = self.stage.space.camera;
        match self.workspace {
            Workspace::Home | Workspace::Show => None,
            Workspace::Media => {
                self.media.drag = None;
                self.media.silence();
                let o = self.media.open.take()?;
                own.then(|| Kept::Media(Box::new(o)))
            }
            Workspace::Module => {
                self.module.autosave(true, std::time::Instant::now());
                self.module.leave(&mut self.stage);
                if !own {
                    return None;
                }
                let m = &mut self.module;
                Some(Kept::Module(Box::new(ModuleKept { open: m.open.take(), descriptor: m.descriptor.take(), problems: std::mem::take(&mut m.problems), auto: std::mem::take(&mut m.auto) })))
            }
            Workspace::Stage if self.scene.is_some() => {
                // a member open as its rig goes back into the scene first
                if self.scene.as_ref().is_some_and(|v| v.inside.is_some()) {
                    self.scene_up();
                }
                let view = self.scene.take()?;
                Some(Kept::Scene(Box::new(SceneKept { view, camera, wire: self.wire })))
            }
            Workspace::Stage => Some(Kept::Rig(Box::new(RigKept {
                bundle: self.stage.bundle.clone(),
                base: self.stage.base.clone(),
                subject: self.rigs.subject.clone(),
                auto: std::mem::take(&mut self.rigs.auto),
                draft: std::mem::replace(&mut self.draft, rigs::Copy::Nothing),
                undo: std::mem::take(&mut self.undo),
                sel: std::mem::replace(&mut self.sel, inspector::Sel::Rig),
                path: self.stage.path.clone(),
                wire: self.wire,
                camera,
            }))),
        }
    }

    /// the front tab goes behind into its place in the list
    fn put_behind(&mut self) {
        let dirty = self.live_dirty();
        let kept = self.freeze();
        if let Some(t) = self.tabs.front_mut() {
            (t.kept, t.dirty) = (kept, dirty);
        } else if let Some(Kept::Scene(s)) = kept {
            self.show.keep_unsent(s.view.edit.doc, s.view.auto);
        }
    }

    /// what a tab kept is let go: a scene's copy not sent yet stays with Show, as when it was closed before the tabs
    fn let_go(&mut self, gone: Vec<tabs::Tab<Kept>>) {
        for t in gone {
            if let Some(k) = t.kept {
                self.let_go_kept(k);
            }
        }
    }

    /// tab `i` comes forward: the front one goes behind, `i` takes the stage as it was left
    pub(crate) fn activate(&mut self, i: usize) {
        if self.tabs.front == Some(i) || i >= self.tabs.list.len() {
            return;
        }
        self.put_behind();
        self.tabs.set_front(i);
        self.thaw();
        self.remember_tabs();
    }

    /// the front tab's item takes the stage: from what it kept, else from its address
    fn thaw(&mut self) {
        let Some(f) = self.tabs.front else { return self.go_home() };
        let (key, kind) = (self.tabs.list[f].key.clone(), self.tabs.list[f].kind);
        let kept = self.tabs.list[f].kept.take();
        let (x, y) = self.cursor;
        match kept {
            Some(Kept::Rig(r)) => {
                self.workspace = Workspace::Stage;
                self.stage.path.clone_from(&r.path);
                if let Err(err) = self.stage.replay(r.bundle) {
                    self.refuse(format!("{err:#}"), x, y);
                }
                self.stage.base = r.base;
                self.stage.space.camera = r.camera;
                (self.rigs.subject, self.rigs.auto, self.draft, self.undo, self.sel, self.wire) = (r.subject, r.auto, r.draft, r.undo, r.sel, r.wire);
                self.leveled = !self.wire;
                self.armed = None;
            }
            Some(Kept::Scene(s)) => {
                self.workspace = Workspace::Stage;
                self.stage.path.clear();
                self.scene = Some(s.view);
                self.replay_scene();
                self.stage.space.camera = s.camera;
                self.wire = s.wire;
                (self.undo, self.sel, self.armed) = (Vec::new(), inspector::Sel::Rig, None);
            }
            Some(Kept::Module(m)) => {
                self.workspace = Workspace::Module;
                self.wake_stage();
                let md = &mut self.module;
                (md.open, md.descriptor, md.problems, md.auto) = (m.open, m.descriptor, m.problems, m.auto);
            }
            Some(Kept::Show(host)) => {
                self.wake_stage();
                self.take_host(*host);
                self.workspace = Workspace::Show;
            }
            Some(Kept::Media(o)) => {
                self.workspace = Workspace::Media;
                self.stage.rest();
                self.media.open = Some(*o);
                self.media_sound();
            }
            None => self.load(&key, kind),
        }
    }

    /// a tab with nothing kept (a later start's, one replaced in place) opens from its address
    fn load(&mut self, key: &str, kind: Kind) {
        let (x, y) = self.cursor;
        match kind {
            Kind::Show => {
                self.wake_stage();
                if self.live_show_key() != key {
                    let host = self.new_host(key.trim_start_matches("show:"));
                    self.take_host(host);
                }
                self.workspace = Workspace::Show;
            }
            Kind::Module => {
                self.workspace = Workspace::Module;
                self.wake_stage();
                let id = key.trim_start_matches("module:");
                if !id.is_empty() && self.module.script.open.as_deref() != Some(id) {
                    self.library_open = Some(id.to_string());
                }
            }
            Kind::Character => {
                self.workspace = Workspace::Stage;
                self.open_rig_now(&subject_of(key));
            }
            Kind::Scene => {
                // asked from the server; until it comes the stage is silent
                self.workspace = Workspace::Stage;
                self.idle_stage();
                self.show.open_scene(key);
            }
            Kind::Media => self.media_load(key, None, deck_core::trim::Trim::WHOLE),
            Kind::Item | Kind::Emotion => {
                self.workspace = Workspace::Stage;
                self.refuse(format!("{key}: no Asset view for a {} yet", kind.word()), x, y);
            }
        }
    }

    /// a rig takes the stage: the server's (with the local copy) when the catalog is there, else the deck's own
    fn open_rig_now(&mut self, subject: &str) {
        let (x, y) = self.cursor;
        self.drop_scene();
        if self.autosaves && self.rigs.open(subject).is_ok() {
            return;
        }
        match rigs::base(&crate::cache_dir(), subject) {
            Ok((base, at)) => {
                self.stage.path.clear();
                if let Err(err) = self.stage.replay(base.clone()) {
                    return self.refuse(format!("{subject}: {err:#}"), x, y);
                }
                self.stage.base = base;
                (self.rigs.subject, self.rigs.auto) = (subject.to_string(), Autosave::new(at));
                (self.undo, self.sel, self.armed, self.draft) = (Vec::new(), inspector::Sel::Rig, None, rigs::Copy::Nothing);
            }
            Err(e) => self.refuse(format!("{subject}: {e:#}"), x, y),
        }
    }

    /// An item just loaded is about to take the stage: the front tab goes behind first (keeping its state), then the
    /// item's tab comes forward – a new one, unless it is open (then that one, with its kept state dropped: what loaded
    /// is newer).
    pub(crate) fn adopt(&mut self, key: &str, name: &str) {
        if self.tabs.front().is_some_and(|t| t.key == key) {
            return;
        }
        self.put_behind();
        let gone = self.tabs.add(key, name);
        self.let_go(gone);
        if let Some(stale) = self.tabs.front_mut().and_then(|t| t.kept.take()) {
            self.let_go_kept(stale);
        }
        self.remember_tabs();
    }

    fn let_go_kept(&mut self, kept: Kept) {
        if let Kept::Scene(s) = kept {
            self.show.keep_unsent(s.view.edit.doc, s.view.auto);
        }
    }

    /// an item from Home, Recent, the library or a check run: its tab forward when it is open, else it opens in a new one
    pub(crate) fn open_item(&mut self, key: &str, name: &str) {
        if let Some(i) = self.tabs.find(key) {
            return self.activate(i);
        }
        let (x, y) = self.cursor;
        let Some(kind) = Kind::of(key) else { return self.refuse(format!("{key}: nothing that opens as a tab"), x, y) };
        match kind {
            // these load first, then adopt (open_scene, follow_rigs)
            Kind::Scene => self.show.open_scene(key),
            Kind::Character if self.autosaves && self.rigs.open(&subject_of(key)).is_ok() => {}
            _ => {
                self.adopt(key, name);
                self.load(key, kind);
            }
        }
    }

    /// tab `i` closed: its edits sent, its state let go; the neighbour comes forward, the last one leaves Home
    pub(crate) fn close_tab(&mut self, i: usize) {
        if i >= self.tabs.list.len() {
            return;
        }
        let front = self.tabs.front == Some(i);
        if front {
            self.let_go_live();
        }
        let Some((tab, _)) = self.tabs.close(i) else { return };
        self.let_go(vec![tab]);
        if front {
            self.thaw();
        }
        self.remember_tabs();
    }

    /// the front item is let go: sent, and the editor's own state of it cleared
    fn let_go_live(&mut self) {
        self.stop_play();
        self.leave_view();
        match self.workspace {
            Workspace::Stage => self.drop_scene(),
            Workspace::Module => {
                self.module.autosave(true, std::time::Instant::now());
                self.module.leave(&mut self.stage);
                self.module.open = None;
                self.module.problems.clear();
            }
            Workspace::Show => {
                self.show.autosave(true, std::time::Instant::now());
                let empty = self.new_host("");
                self.take_host(empty);
            }
            Workspace::Media => {
                self.media.drag = None;
                self.media.silence();
                self.media.open = None;
            }
            Workspace::Home => {}
        }
    }

    /// A medium's trim onto its use: the scene's `video` node `start` / `end` in that scene's tab (behind the medium's),
    /// sent with the scene when it saves. Err: the scene is not open, or the node has gone.
    pub(crate) fn trim_use(&mut self, used: &crate::media::Use, trim: deck_core::trim::Trim) -> Result<(), String> {
        let Some(i) = self.tabs.find(&used.scene) else { return Err(format!("{} is not open: open it to trim its video", used.scene)) };
        let Some(Kept::Scene(s)) = self.tabs.list[i].kept.as_mut() else { return Err(format!("{} is still loading", used.scene)) };
        let e = &mut s.view.edit;
        let v = e.doc.nodes.iter().position(|n| n.id == used.node && n.kind == "video").ok_or_else(|| format!("{}: no video node {}", used.scene, used.node))?;
        e.set_video(v, "start", trim.start as f32)?;
        e.set_video(v, "end", trim.end as f32)?;
        s.view.refresh()?;
        self.tabs.list[i].dirty = true;
        Ok(())
    }

    pub(crate) fn close_others(&mut self) {
        let Some(f) = self.tabs.front else { return };
        let gone = self.tabs.close_others(f);
        self.let_go(gone);
        self.remember_tabs();
    }

    pub(crate) fn close_all(&mut self) {
        if self.tabs.front.is_some() {
            self.let_go_live();
        }
        let gone = self.tabs.close_all();
        self.let_go(gone);
        self.go_home();
        self.remember_tabs();
    }

    /// Home: the front tab goes behind, the idle scene plays
    pub(crate) fn home(&mut self) {
        if self.tabs.front.is_none() {
            return;
        }
        self.put_behind();
        self.tabs.front = None;
        self.go_home();
        self.remember_tabs();
    }

    fn go_home(&mut self) {
        self.workspace = Workspace::Home;
        self.scene = None;
        self.idle_stage();
    }

    /// Home's stage is silent; its picture is the idle scene's (idle.rs)
    fn idle_stage(&mut self) {
        self.stage.rest();
    }

    /// back from Home to a view that plays no item of its own: what the stage held sounds again
    fn wake_stage(&mut self) {
        if self.stage.resting() {
            let (x, y) = self.cursor;
            if let Err(err) = self.stage.replay(self.stage.bundle.clone()) {
                self.refuse(format!("{err:#}"), x, y);
            }
        }
    }

    /// the address of the show the editor's host holds
    pub(crate) fn live_show_key(&self) -> String {
        format!("show:{}", self.show.sub.conductor().show().id)
    }

    /// a show's host of its own: a demo of that name, else the server's show (empty: a new show)
    fn new_host(&self, id: &str) -> show::Host {
        let api = self.args.api.clone();
        let mut host = show::Host::new(None, None, None, api.clone());
        if !id.is_empty() && host.open_demo(id).is_err() {
            host = show::Host::new(None, Some(id.to_string()), None, api);
        }
        host.toon.clone_from(&self.show.toon);
        host.nodes.direction = self.show.nodes.direction;
        host.knobs.rigs.clone_from(&self.show.knobs.rigs);
        host.show_panels.floats.restore(&self.show.show_panels.floats.places());
        host
    }

    /// `host` becomes the editor's; the one it held goes to its own tab behind, when it has one
    fn take_host(&mut self, host: show::Host) {
        let old = std::mem::replace(&mut self.show, host);
        let key = format!("show:{}", old.sub.conductor().show().id);
        if let Some(i) = self.tabs.find(&key).filter(|i| Some(*i) != self.tabs.front) {
            self.tabs.list[i].kept = Some(Kept::Show(Box::new(old)));
        }
    }

    /// Show's own host to the front: its tab, or a new one
    pub(crate) fn go_show(&mut self) {
        let key = self.live_show_key();
        match self.tabs.find(&key) {
            Some(i) => self.activate(i),
            None => {
                let name = tabs::name_of(&key);
                self.adopt(&key, &name);
                self.workspace = Workspace::Show;
            }
        }
    }

    /// Each frame: an item replaced in place without its tab knowing – a show or a module picked in its empty tab names
    /// it; another show loaded (a demo, a new show) or another module picked in a tab that held one gets a tab of its
    /// own, in front (the other reopens from its address). The stage's items come through `adopt`.
    pub(crate) fn follow_tabs(&mut self) {
        let Some(f) = self.tabs.front else { return };
        let (key, name) = match self.workspace {
            Workspace::Show => match self.live_key() {
                Some(k) => k,
                None => return,
            },
            Workspace::Module => match &self.module.open {
                Some(o) => (o.key(), o.id.clone()),
                None => return,
            },
            _ => return,
        };
        if self.tabs.list[f].key == key {
            return;
        }
        let empty = if self.workspace == Workspace::Module { NEW_MODULE } else { EMPTY_SHOW };
        if self.tabs.list[f].key == empty {
            self.tabs.rekey(f, &key, &name);
        } else {
            self.tabs.list[f].kept = None;
            let gone = self.tabs.add(&key, &name);
            self.let_go(gone);
            // what loaded is newer than what its tab kept, if it was open
            if let Some(t) = self.tabs.front_mut() {
                t.kept = None;
            }
        }
        self.remember_tabs();
    }

    /// Quitting: the saves of the tabs behind (sent as each went behind) are waited for too, until `until`; a show
    /// behind sends what it has not sent first
    pub(crate) fn wait_behind(&mut self, until: std::time::Instant) {
        let now = std::time::Instant::now();
        for t in &mut self.tabs.list {
            match t.kept.as_mut() {
                Some(Kept::Rig(r)) => _ = r.auto.wait(until),
                Some(Kept::Scene(s)) => _ = s.view.auto.wait(until),
                Some(Kept::Module(m)) => _ = m.auto.wait(until),
                Some(Kept::Show(h)) => {
                    h.autosave(true, now);
                    h.wait_saves(until);
                }
                Some(Kept::Media(_)) | None => {}
            }
        }
    }

    /// what a later start reopens (none in a check run)
    pub(crate) fn remember_tabs(&self) {
        if let Some(f) = &self.tabs_file {
            tabs::save(f, &self.tabs.saved());
        }
    }

    /// The first frame: the tabs the flags name, else the last start's (`tabs_file`; a check run has one only with
    /// `--tabs-file`); none is Home. The stage holds the start's rig already, so its tab is the one in front before the
    /// asked one comes forward.
    pub(crate) fn start_tabs(&mut self) {
        let mut keys: Vec<String> = Vec::new();
        let rig = rig_key(&self.rigs.subject).0;
        let named = self.args.show_file.is_some() || self.args.show_id.is_some();
        let shows = opens_show(named, self.show.unloaded.is_some(), self.args.demo.is_some(), self.args.workspace);
        if let Some(why) = &self.show.unloaded {
            crate::log::post(deck_ui::messages::Level::Warn, "editor", format!("no show opened – {why}"));
        }
        let modules = self.args.module.open.is_some() || self.args.workspace == Workspace::Module;
        if self.args.rig_given {
            keys.push(rig.clone());
        }
        for k in self.args.tabs.clone().unwrap_or_default() {
            if k != "home" && !keys.contains(&k) {
                keys.push(k);
            }
        }
        let module = self.args.module.open.as_ref().map_or_else(|| NEW_MODULE.to_string(), |id| format!("module:{id}"));
        if modules {
            keys.push(module.clone());
        }
        if shows {
            keys.push(self.live_show_key());
        }
        if self.tabs_file.is_some() && keys.is_empty() && self.args.tabs.is_none() && self.args.scene_file.is_none() {
            // a later start: the last tabs, or Home
            let saved = self.tabs_file.as_deref().map(tabs::load).unwrap_or_default();
            self.tabs = tabs::Tabs::from_saved(saved, self.tabs.cap);
            let front = self.tabs.front.take();
            self.tabs.front = self.tabs.find(&rig);
            match front {
                Some(i) if self.tabs.front != Some(i) => {
                    self.workspace = Workspace::Stage;
                    self.activate(i);
                }
                Some(_) => self.workspace = Workspace::Stage,
                None => {
                    self.tabs.front = None;
                    self.go_home();
                }
            }
            return;
        }
        for k in &keys {
            let name = if k == NEW_MODULE { "new module".to_string() } else { tabs::name_of(k) };
            let gone = self.tabs.add(k, &name);
            self.let_go(gone);
        }
        // the one asked for, else the last named; the rig the stage holds is the one in front until then
        let want = match &self.args.front {
            Some(f) => f.parse::<usize>().ok().filter(|i| *i < self.tabs.list.len()).or_else(|| self.tabs.find(f)),
            None if modules => self.tabs.find(&module),
            None if shows => self.tabs.find(&self.live_show_key()),
            None => self.tabs.list.len().checked_sub(1),
        };
        self.tabs.front = self.tabs.find(&rig);
        self.workspace = if self.tabs.front.is_some() { Workspace::Stage } else { Workspace::Home };
        match want {
            Some(i) if self.tabs.front != Some(i) => {
                if self.tabs.front.is_none() {
                    // nothing of the start's to keep
                    self.tabs.set_front(i);
                    self.thaw();
                } else {
                    self.activate(i);
                }
            }
            Some(_) => {}
            None => self.go_home(),
        }
        self.remember_tabs();
    }

    /// the toolbar's tabs this frame: each one's pill, the front one's read live
    pub(crate) fn tab_pills(&self) -> Vec<(String, &'static str, bool, bar::Mark)> {
        let live = self.show.relay().map(|_| self.live_show_key());
        self.tabs
            .list
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let front = self.tabs.front == Some(i);
                let dirty = if front { self.live_dirty() } else { t.dirty || matches!(&t.kept, Some(Kept::Show(h)) if show_dirty(h)) };
                let playing = front && match self.workspace {
                    Workspace::Stage => self.playing(),
                    Workspace::Module => self.module.heard,
                    Workspace::Media => self.playing(),
                    _ => false,
                };
                let mark = if live.as_deref() == Some(t.key.as_str()) || matches!(&t.kept, Some(Kept::Show(h)) if h.live()) {
                    bar::Mark::Live
                } else if playing {
                    bar::Mark::Playing
                } else {
                    bar::Mark::None
                };
                (t.name.clone(), t.kind.icon(), dirty, mark)
            })
            .collect()
    }

    /// what Home lists: the recent items, the library (what opens as a tab), the deck's state
    pub(crate) fn home_lines(&self) -> (Vec<home::Line>, Vec<home::Line>, Vec<String>) {
        let recent = self.tabs.recent.iter().map(|(k, n)| home::Line { key: k.clone(), name: n.clone() }).collect();
        let mut library: Vec<home::Line> = Vec::new();
        let entries = self.show.library_entries().into_iter().chain(crate::library::of_modules(&self.module.list));
        for e in entries {
            let opens = Kind::of(&e.address).is_some_and(|k| matches!(k.view(), View::Scene | View::Asset));
            if opens && !library.iter().any(|l| l.key == e.address) {
                library.push(home::Line { key: e.address.clone(), name: e.label.clone() });
            }
        }
        let account = match (self.auth.state(), self.auth.name()) {
            (crate::sign_in::Signed::In, Some(n)) => format!("signed in as {n}"),
            (crate::sign_in::Signed::In, None) => "signed in".to_string(),
            (crate::sign_in::Signed::Signing, _) => "signing in …".to_string(),
            (crate::sign_in::Signed::Out, _) => "not signed in".to_string(),
        };
        let audio = if self.stage.audio.is_empty() { "audio: none".to_string() } else { format!("audio: {}", self.stage.audio) };
        let relay = match self.show.relay() {
            Some(r) => format!("relay: {r}"),
            None => "relay: not joined".to_string(),
        };
        (recent, library, vec![account, audio, relay])
    }

    /// a press on Home
    pub(crate) fn home_press(&mut self, p: home::Press) {
        let (x, y) = self.cursor;
        let (recent, library, _) = self.home_lines();
        match p {
            home::Press::Recent(i) => {
                if let Some(l) = recent.get(i) {
                    self.open_item(&l.key, &l.name);
                }
            }
            home::Press::Library(i) => {
                if let Some(l) = library.get(i) {
                    self.open_item(&l.key, &l.name);
                }
            }
            // the system's browser on a desktop (a check run opens none); its link copied where none opens
            home::Press::Docs if self.autosaves && home::open_in_browser(home::DOCS).is_ok() => self.note(format!("{} opens in the browser", home::DOCS)),
            home::Press::Docs => match self.clipboard.as_mut().map(|c| c.set_text(home::DOCS)) {
                Some(Ok(())) => self.note(format!("{} copied: no browser opened here", home::DOCS)),
                _ => self.refuse(format!("the help is at {}", home::DOCS), x, y),
            },
            home::Press::New(home::New::Module) => {
                self.adopt(NEW_MODULE, "new module");
                self.workspace = Workspace::Module;
                self.note("a new module starts from one of the catalog's: pick it, it saves as your own".to_string());
            }
            home::Press::New(home::New::Show) => {
                self.new_show_tab();
                self.show.press(show::Hit::Sheet(show::Sheet::NewShow));
            }
            home::Press::New(home::New::Scene) => self.ask_new_scene(),
        }
    }

    /// the prompt for a new scene's name and id; the scene list's ids are the taken ones
    pub(crate) fn ask_new_scene(&mut self) {
        let mut taken: Vec<String> = self.show.scene_list().iter().filter_map(|p| p.address.strip_prefix("scene:").map(String::from)).collect();
        taken.extend(self.tabs.list.iter().filter_map(|t| t.key.strip_prefix("scene:").map(String::from)));
        self.scene_prompt = Some(crate::scene_prompt::ScenePrompt::new(taken));
    }

    /// a key while the new scene's prompt is open
    pub(crate) fn scene_prompt_key(&mut self, k: &winit::event::KeyEvent) {
        if k.state != winit::event::ElementState::Pressed {
            return;
        }
        let (ctrl, shift) = (self.mods.control_key() || self.mods.super_key(), self.mods.shift_key());
        let Some(p) = self.scene_prompt.as_mut() else { return };
        let outcome = p.key(k.logical_key.as_ref(), k.text.as_deref(), ctrl, shift);
        self.scene_prompt_done(outcome);
    }

    pub(crate) fn scene_prompt_done(&mut self, outcome: crate::scene_prompt::Outcome) {
        use crate::scene_prompt::Outcome;
        match outcome {
            Outcome::Nothing => {}
            Outcome::Cancel => self.scene_prompt = None,
            Outcome::Create(id, name) => {
                self.scene_prompt = None;
                self.new_scene_tab(&id, &name);
            }
        }
    }

    /// a new scene, empty, named and with its id, in a Scene tab in front: its cast comes from the library; it saves itself
    /// as a draft once it has one
    pub(crate) fn new_scene_tab(&mut self, id: &str, name: &str) {
        let doc = deck_space::scene_document::SceneDocument::new(id, name, self.show.story_toon().into_iter().collect());
        self.open_scene(show::build::SceneOpen { doc, base: None, kept: None, note: Some("new: drag its cast in from the library".into()), save: None });
    }

    /// a new show of its own, in a tab in front
    fn new_show_tab(&mut self) {
        self.put_behind();
        let host = self.new_host("");
        self.take_host(host);
        let key = self.live_show_key();
        let gone = self.tabs.add(&key, &tabs::name_of(&key));
        self.let_go(gone);
        if let Some(t) = self.tabs.front_mut() {
            t.kept = None;
        }
        self.workspace = Workspace::Show;
        self.remember_tabs();
    }

    /// the path inside the front tab (after its name) and what each step and a press on the tab itself go back to
    pub(crate) fn tab_path(&self) -> (Vec<String>, usize) {
        match self.workspace {
            Workspace::Stage => match self.scene.as_ref().and_then(|v| v.inside.and_then(|m| v.edit.members.get(m))) {
                Some(m) => (std::iter::once(m.holder.clone()).chain(self.stage.path.iter().cloned()).collect(), 0),
                None => (self.stage.path.clone(), 1),
            },
            Workspace::Show => {
                let row = self.show.shown().rows.get(self.show.row).map(|r| format!("{} {}", self.show.row + 1, r.id)).unwrap_or_default();
                (self.show.crumbs().into_iter().skip(1).chain(std::iter::once(row).filter(|r| !r.is_empty())).collect(), 1)
            }
            _ => (Vec::new(), 1),
        }
    }

    /// a press on the front tab itself: back to the item's top (the rig, the scene, the show)
    pub(crate) fn tab_top(&mut self) {
        match self.workspace {
            Workspace::Stage if self.scene.as_ref().is_some_and(|v| v.inside.is_some()) => self.scene_up(),
            Workspace::Stage => self.up_to(0),
            Workspace::Show => self.show.press(show::Hit::Crumb(0)),
            _ => {}
        }
    }
}

/// a show's documents not all on the server
fn show_dirty(h: &show::Host) -> bool {
    h.saves.values().any(|a| a.pending() || a.state != crate::autosave::State::Clean) || !h.unsent.is_empty()
}

/// whether the start opens a Show tab: a show that was named and did not load opens none (Home says why)
fn opens_show(named: bool, unloaded: bool, demo: bool, workspace: Workspace) -> bool {
    !(named && unloaded) && (named || demo || matches!(workspace, Workspace::Show))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_show_that_did_not_load_opens_no_tab() {
        assert!(!opens_show(true, true, false, Workspace::Show));
        assert!(opens_show(true, false, false, Workspace::Stage));
        assert!(opens_show(false, false, false, Workspace::Show));
        assert!(!opens_show(false, false, false, Workspace::Stage));
    }
}
