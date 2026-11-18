//! The catalog modules a rig in Build plays: each one's published build fetched on a thread and loaded the way a deck
//! loads it (deck-native `Library::published`, the files kept in `<cache>/modules`), the module's answer kept beside them
//! so it plays offline. What to ask and why a module is silent are the core's (deck-space rig/build.rs).
use deck_native::module_host::Library;
use deck_space::rig::{build, Bundle};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

type Ask = Arc<dyn Fn(&str) -> Result<Value, String> + Send + Sync>;
type Get = Arc<dyn Fn(&str) -> Result<Vec<u8>, String> + Send + Sync>;

#[derive(Debug, Clone, PartialEq)]
enum State {
    Fetching,
    Loaded,
    Silent(String),
}

pub struct Published {
    dir: PathBuf,
    ask: Ask,
    get: Get,
    library: &'static Library,
    state: BTreeMap<String, State>,
    tx: Sender<(String, Result<(), String>)>,
    rx: Receiver<(String, Result<(), String>)>,
}

impl Published {
    /// against the API `api` (the editor's sign-in), kept in `cache`/modules
    pub fn new(api: &str, cache: &Path) -> Published {
        let (a, g) = (Arc::new(crate::show::api::Api::new(api.to_string())), Arc::new(crate::show::api::Api::new(api.to_string())));
        Published::with(cache.join("modules"), Arc::new(move |id| a.module(id)), Arc::new(move |url| g.bytes(url)), Library::global())
    }

    fn with(dir: PathBuf, ask: Ask, get: Get, library: &'static Library) -> Published {
        let (tx, rx) = channel();
        Published { dir, ask, get, library, state: BTreeMap::new(), tx, rx }
    }

    /// Fetches what `bundle` plays and is not loaded; true when a module arrived since the last call (the stage replays)
    pub fn follow(&mut self, bundle: &Bundle) -> bool {
        for id in build::needed(bundle) {
            if self.state.contains_key(&id) || self.library.get(&id).is_some() {
                continue;
            }
            self.state.insert(id.clone(), State::Fetching);
            let (dir, ask, get, library, tx) = (self.dir.clone(), self.ask.clone(), self.get.clone(), self.library, self.tx.clone());
            std::thread::spawn(move || {
                let r = load(&id, &dir, &*ask, &*get, library);
                let _ = tx.send((id, r));
            });
        }
        let mut arrived = false;
        while let Ok((id, r)) = self.rx.try_recv() {
            match r {
                Ok(()) => {
                    arrived = true;
                    self.state.insert(id, State::Loaded);
                }
                Err(why) => {
                    tracing::warn!("{why}");
                    self.state.insert(id, State::Silent(why));
                }
            }
        }
        arrived
    }

    /// why a track of `bundle` is silent, while its module is fetched or when it has none to play
    pub fn why(&self, bundle: &Bundle, track: &str) -> Option<String> {
        let t = bundle.rig.tracks.iter().find(|t| t.id == track)?;
        let inst = bundle.instruments.iter().find(|i| i.id == t.instrument)?;
        inst.nodes.iter().filter_map(|n| n.kind.strip_prefix(deck_space::modreg::PREFIX)).find_map(|id| match self.state.get(id) {
            Some(State::Fetching) => Some(format!("module:{id}: fetching its build")),
            Some(State::Silent(why)) => Some(why.clone()),
            _ => None,
        })
    }
}

impl crate::App {
    /// the rig's catalog modules: one that arrived replays the rig; the silent ones say why on their tracks
    pub(crate) fn follow_modules(&mut self) {
        if self.published.follow(&self.stage.bundle) && self.stage.scene.is_none() {
            if let Err(err) = self.stage.replay(self.stage.bundle.clone()) {
                let (x, y) = self.cursor;
                self.refuse(format!("{err:#}"), x, y);
            }
        }
        let silent = self.stage.bundle.rig.tracks.iter().filter_map(|t| Some((t.id.clone(), self.published.why(&self.stage.bundle, &t.id)?))).collect();
        self.stage.silent = silent;
    }
}

/// module `id`'s build into `library`: the API's answer (else the one kept), then the build's file (else fetched)
fn load(id: &str, dir: &Path, ask: &dyn Fn(&str) -> Result<Value, String>, get: &dyn Fn(&str) -> Result<Vec<u8>, String>, library: &Library) -> Result<(), String> {
    let kept = dir.join(format!("{id}.json"));
    let answer = match ask(id) {
        Ok(v) => {
            let _ = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&kept, v.to_string()));
            v
        }
        Err(e) => std::fs::read_to_string(&kept).ok().and_then(|t| serde_json::from_str(&t).ok()).ok_or(e)?,
    };
    let b = build::of(id, &answer)?;
    library
        .published(&b.hash, dir, || {
            let url = b.url.as_deref().ok_or_else(|| anyhow::anyhow!("the API named no file for it"))?;
            get(url).map_err(anyhow::Error::msg)
        })
        .map(|_| ())
        .map_err(|e| format!("module:{id}: {e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::OnceLock;

    fn library() -> &'static Library {
        static L: OnceLock<Library> = OnceLock::new();
        L.get_or_init(|| Library::with_cache(&std::env::temp_dir().join(format!("deck-editor-published-compiled-{}", std::process::id()))))
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("deck-editor-published-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn rig(module: &str) -> Bundle {
        let doc = json!({ "id": module, "descriptor": { "name": module, "ports": [{ "name": "out", "dir": "out", "type": "audio" }] } });
        let (inst, d) = deck_space::rig::assign::module_track(&doc).unwrap();
        Bundle::empty("character.boy").module_track_added(&inst, &d).unwrap().0
    }

    fn settle(p: &mut Published, b: &Bundle) {
        for _ in 0..500 {
            p.follow(b);
            if !p.state.values().any(|s| *s == State::Fetching) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("still fetching");
    }

    #[test]
    fn a_module_without_a_published_build_says_so_on_its_track() {
        let d = dir("unbuilt");
        let mut p = Published::with(d.clone(), Arc::new(|id| Ok(json!({ "module": { "id": id, "files": {} } }))), Arc::new(|_| panic!("nothing to fetch")), library());
        let b = rig("drone_x");
        p.follow(&b);
        assert_eq!(p.why(&b, "drone_x").as_deref(), Some("module:drone_x: fetching its build"));
        settle(&mut p, &b);
        assert_eq!(p.why(&b, "drone_x"), Some(build::unbuilt("drone_x")));
        assert_eq!(p.why(&b, "nope"), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn offline_the_kept_answer_names_the_build_and_wrong_bytes_are_refused() {
        let d = dir("offline");
        let hash = "ab".repeat(32);
        let answer = json!({ "module": { "id": "pad_x", "wasm": { "hash": hash, "bytes": 4, "url": "https://cdn.test/modules/x.wasm" } } });
        let online = answer.clone();
        let mut p = Published::with(d.clone(), Arc::new(move |_| Ok(online.clone())), Arc::new(|_| Ok(b"\0asm".to_vec())), library());
        let b = rig("pad_x");
        settle(&mut p, &b);
        assert!(p.why(&b, "pad_x").unwrap().starts_with("module:pad_x: "), "not a module: silent, with the loader's words");
        assert!(d.join("pad_x.json").exists(), "the answer kept");
        // offline: the kept answer is read, the same build asked for
        let asked = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = asked.clone();
        let mut q = Published::with(d.clone(), Arc::new(|_| Err("no network".into())), Arc::new(move |url| {
            seen.lock().unwrap().push(url.to_string());
            Err("no network".into())
        }), library());
        settle(&mut q, &b);
        assert_eq!(*asked.lock().unwrap(), ["https://cdn.test/modules/x.wasm"]);
        assert!(q.why(&b, "pad_x").unwrap().contains("no network"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
