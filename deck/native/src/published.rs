//! A published scene on the native deck (#246, plan 2a): `--scene scene:<id>[@v]` and deck.toml's `scene = "scene:<id>"`
//! (deck-session hands it over as `--scene`). The loader is the core's (deck-space published.rs: what to ask, the cache
//! rule by version, the resolve step, the reason when it does not play); this file only moves its bytes – the GETs to the
//! API (with the deck token when there is one, #180) and one file per scene under `<cache>/scenes/`, so a scene fetched
//! once plays offline after a restart. A scene that does not play is an error with the core's words: the deck plays the
//! test pattern and shows it (main.rs `load_or_explain`). Its set and its members' pictures come through the sync as any
//! scene's (sync.rs).
use anyhow::{anyhow, Result};
use deck_space::published::{Load, Loaded, SceneRef, Step};
use deck_space::scene::Scene;
use std::path::Path;

/// how many asks one load takes at most (the document and scene-content: two)
const MAX_ASKS: usize = 4;

/// What a scene argument names as a published scene (`scene:nave`, `scene:nave@3`); None: not one.
pub fn named(arg: &Path) -> Option<Result<SceneRef, String>> {
    SceneRef::parse(arg.to_str()?)
}

/// The published scene `r`: the core's loader driven with `get` (a path after the API's base → its HTTP status and body,
/// 0 = the server can't be reached) and the cache folder `dir` (`<cache>/scenes`).
pub fn load(r: SceneRef, dir: &Path, mut get: impl FnMut(&str) -> (u16, String)) -> Result<Loaded> {
    let file = dir.join(r.cache_name());
    let cached = std::fs::read_to_string(&file).ok();
    let (mut load, mut step) = Load::start(r, cached.as_deref());
    for _ in 0..MAX_ASKS {
        match step {
            Step::Ask(path) => {
                let (status, body) = get(&path);
                step = load.answer(status, &body);
            }
            Step::Play(l) => {
                if let Some(text) = &l.store {
                    if let Err(e) = write_atomic(&file, text.as_bytes()) {
                        tracing::warn!("scene: {}: not kept: {e}", load.reference());
                    }
                }
                for note in &l.notes {
                    tracing::info!("scene: {}: {note}", load.reference());
                }
                tracing::info!("scene: {} version {}{}", load.reference(), l.version, if l.cached { " from the cache" } else { "" });
                return Ok(*l);
            }
            Step::Fail(why) => return Err(anyhow!(why)),
        }
    }
    Err(anyhow!("{}: the loader keeps asking", load.reference()))
}

/// The scene `arg` names (`named`), fetched from `api` into `<cache>/scenes/` – or from there when the API can't be reached.
pub fn scene(arg: &Path, api: &str, cache: &Path) -> Result<Scene> {
    let r = named(arg).ok_or_else(|| anyhow!("'{}' names no published scene", arg.display()))?.map_err(|e| anyhow!(e))?;
    Ok(fetch(r, api, cache)?.scene)
}

/// The published scene `r` from `api` into `<cache>/scenes/` – or from there when the API can't be reached (a flash's
/// check, #513: live/flash.rs).
pub fn fetch(r: SceneRef, api: &str, cache: &Path) -> Result<Loaded> {
    load(r, &cache.join("scenes"), |path| get(api, path))
}

/// #513: the published scenes this deck keeps (`<cache>/scenes/`), for F5: each as `scene:<id>@<v>` (the kept version
/// plays without asking) and its stage's name – the core reads the file (deck-space `published::listed`) – by name
pub fn cached(cache: &Path) -> Vec<(SceneRef, String)> {
    let files = std::fs::read_dir(cache.join("scenes")).into_iter().flatten().flatten().map(|e| e.path());
    let mut out: Vec<(SceneRef, String)> = files.filter(|p| p.extension().is_some_and(|x| x == "json")).filter_map(|p| deck_space::published::listed(&std::fs::read_to_string(p).ok()?)).collect();
    out.sort_by(|a, b| (a.1.to_lowercase(), &a.0.id).cmp(&(b.1.to_lowercase(), &b.0.id)));
    out
}

/// GET `<api><path>` → (status, body); 0 = the network is gone
#[cfg(feature = "sync")]
fn get(api: &str, path: &str) -> (u16, String) {
    use std::time::Duration;
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout_read(Duration::from_secs(20)).build();
    get_with(&agent, api, path, deck_link::TokenSource::from_env().get())
}

/// GET `<api><path>` through `agent`, with the deck `token` when there is one (#180) → (status, body); 0 = the network
/// is gone (`loader`: the sync worker's agent and token)
#[cfg(feature = "sync")]
pub fn get_with(agent: &ureq::Agent, api: &str, path: &str, token: Option<String>) -> (u16, String) {
    let url = format!("{}{path}", api.trim_end_matches('/'));
    let mut req = agent.get(&url);
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    match req.call() {
        Ok(r) => (r.status(), r.into_string().unwrap_or_default()),
        Err(ureq::Error::Status(code, r)) => (code, r.into_string().unwrap_or_default()),
        Err(ureq::Error::Transport(t)) => {
            tracing::warn!("scene: {url}: {t} – from the cache");
            (0, String::new())
        }
    }
}

/// #513: the core's loader for the published scenes a conductor's push names (deck-space `push::preload`, the sync worker):
/// through `agent` with the deck `token`, into `<cache>/scenes/` – the copy `--scene` and F5 play
#[cfg(feature = "sync")]
pub fn loader<'a>(agent: &'a ureq::Agent, api: &'a str, cache: &'a Path, token: Option<String>) -> impl FnMut(SceneRef) -> Result<Scene, String> + 'a {
    move |r| {
        let got = load(r, &cache.join("scenes"), |path| get_with(agent, api, path, token.clone())).map(|l| l.scene).map_err(|e| format!("{e:#}"));
        if let Err(why) = &got {
            tracing::warn!("sync: {why}");
        }
        got
    }
}

/// a build without the sync fetches nothing: the cache's copy plays
#[cfg(not(feature = "sync"))]
fn get(_: &str, _: &str) -> (u16, String) {
    (0, String::new())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const NAVE: &str = include_str!("../../../server/tests/fixtures/scenes/nave.json");
    const RIGS: &str = include_str!("../../../server/tests/fixtures/rigs/scene-content.json");

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("deck-scenes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// the API with `scene:nave` published at `version`; every path asked is noted
    fn api(version: u32, asked: &mut Vec<String>) -> impl FnMut(&str) -> (u16, String) + '_ {
        move |path: &str| {
            asked.push(path.to_string());
            if path.starts_with("/scenes/nave") {
                let doc = NAVE.replacen("\"format\": 2,", &format!("\"format\": 2, \"version\": {version},"), 1);
                (200, format!("{{\"status\":\"success\",\"scene\":{doc}}}"))
            } else if path.starts_with("/library/scene-content") {
                (200, RIGS.to_string())
            } else {
                (404, "{\"status\":\"error\"}".into())
            }
        }
    }

    fn nave() -> SceneRef {
        named(Path::new("scene:nave")).unwrap().unwrap()
    }

    #[test]
    fn a_scene_argument_names_a_published_scene() {
        assert_eq!(named(Path::new("scene:nave@3")), Some(Ok(SceneRef { id: "nave".into(), version: Some(3) })));
        assert_eq!(named(Path::new("idle")), None);
        assert!(matches!(named(Path::new("scene:../etc")), Some(Err(_))));
    }

    #[test]
    fn a_new_version_replaces_the_cached_one_an_unchanged_one_is_not_fetched_again() {
        let d = dir("versions");
        let mut asked = Vec::new();
        let l = load(nave(), &d, api(4, &mut asked)).unwrap();
        assert_eq!((l.version, l.cached), (4, false));
        assert_eq!(asked, ["/scenes/nave", "/library/scene-content?sets=&locations=&characters=catpope&items="]);
        assert!(d.join("nave.json").is_file(), "kept under <cache>/scenes/");
        // after a restart, unchanged: only the document is asked, the copy plays
        let mut asked = Vec::new();
        let l = load(nave(), &d, api(4, &mut asked)).unwrap();
        assert_eq!((l.version, l.cached, asked.len()), (4, true, 1));
        // a new version is fetched and replaces the copy
        let mut asked = Vec::new();
        let l = load(nave(), &d, api(5, &mut asked)).unwrap();
        assert_eq!((l.version, l.cached, asked.len()), (5, false, 2));
        assert!(std::fs::read_to_string(d.join("nave.json")).unwrap().contains("\"version\":5"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_unreachable_server_plays_the_cached_version() {
        let d = dir("offline");
        load(nave(), &d, api(4, &mut Vec::new())).unwrap();
        let l = load(nave(), &d, |_| (0, String::new())).unwrap();
        assert_eq!((l.version, l.cached), (4, true));
        assert_eq!(l.scene.being_character(), "catpope", "the copy's rig plays");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_unknown_or_broken_scene_is_the_core_s_reason() {
        let d = dir("unknown");
        let e = load(named(Path::new("scene:crypt")).unwrap().unwrap(), &d, api(1, &mut Vec::new())).unwrap_err();
        assert_eq!(e.to_string(), "scene:crypt is not published");
        let e = load(nave(), &d, |_| (0, String::new())).unwrap_err();
        assert_eq!(e.to_string(), "scene:nave: the API can't be reached and this deck has no copy");
        // a broken copy in the cache is as none
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("nave.json"), "not json").unwrap();
        assert!(load(nave(), &d, |_| (0, String::new())).is_err());
        let broken = |_: &str| (200, "{\"status\":\"success\",\"scene\":{\"format\":1,\"id\":\"nave\"}}".to_string());
        assert!(load(nave(), &d, broken).unwrap_err().to_string().starts_with("scene:nave: "));
        assert!(scene(Path::new("scene:nave@x"), "http://127.0.0.1:9", &d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
