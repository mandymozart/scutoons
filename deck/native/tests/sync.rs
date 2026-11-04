//! The deck's sync (#195) against a stub of scutoons.com in-process (127.0.0.1, 7000–7099): one server answers
//! `/api/library/scene-content` and serves the CDN files, counting every request. A scene's needs → one API request →
//! the set's 3D and the portraits in the cache; a second run skips what is cached; a changed path is fetched again; a
//! broken file is refused (the old one stays); offline plays from the cache; a character wanted later is asked for then.
#![cfg(feature = "sync")]
mod common;

use common::*;
use deck_native::sync::{Config, DeckSync, Want};
use std::time::{Duration, Instant};

#[test]
fn a_scene_s_needs_land_in_the_cache_once() {
    let stub = stub_with_files();
    let cache = dir("once");
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    // one request for the whole scene, with the token; then the CDN files (the mood without a sprite: nothing)
    let api = stub.hits("/api/");
    assert_eq!(api.len(), 1, "{api:?}");
    assert!(api[0].contains("sets=cathedral&characters=catpope") && api[0].ends_with("+token"), "{}", api[0]);
    let mut cdn = stub.hits("/cdn/");
    cdn.sort();
    assert_eq!(
        cdn,
        ["/cdn/catpope/map/sets/cathedral.low.v1.glb", "/cdn/characters/catpope/catpope_portrait-happy.png", "/cdn/characters/catpope/catpope_portrait-neutral.png"],
        "only the wanted quality, no token to the CDN"
    );
    let set = cache.join("sets/cathedral/cathedral.low.glb");
    assert_eq!(std::fs::read(&set).unwrap(), glb("low"));
    assert_eq!(deck_native::cache::set_file(&cache.join("sets"), "cathedral", "low"), set, "where the space node reads it");
    assert!(cache.join("portraits/catpope/catpope_portrait-neutral.png").is_file());
    assert!(cache.join("sync/index.json").is_file());
    assert_eq!(sync.line(), "SYNC OK 3 NEW 0 CACHED");
    drop(sync);

    // the next start: asked again, nothing fetched
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    assert_eq!((stub.hits("/api/").len(), stub.hits("/cdn/").len()), (2, 3), "cached files are skipped");
    assert_eq!(sync.line(), "SYNC OK 0 NEW 3 CACHED");
    drop(sync);

    // a changed file has a new path: only it is fetched again
    let files = stub.files.lock().unwrap().clone();
    *stub.api.lock().unwrap() = answer(&files, LOW, "characters/catpope/catpope_portrait-neutral.v2.png", HAPPY, None);
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    assert_eq!(stub.hits("/cdn/").last().unwrap(), "/cdn/characters/catpope/catpope_portrait-neutral.v2.png");
    assert_eq!(stub.hits("/cdn/").len(), 4);
    assert_eq!(sync.line(), "SYNC OK 1 NEW 2 CACHED");
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_broken_file_is_refused_and_the_old_one_stays() {
    let stub = stub_with_files();
    let cache = dir("broken");
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    drop(sync);
    // v2 of the set arrives truncated (the API's size says more), v2 of a portrait is an error page
    {
        let mut files = stub.files.lock().unwrap();
        files.insert("catpope/map/sets/cathedral.low.v2.glb".into(), glb("lo"));
        files.insert("characters/catpope/catpope_portrait-happy.v2.png".into(), b"<html>502</html>".to_vec());
    }
    let files = stub.files.lock().unwrap().clone();
    let (low2, happy2) = ("catpope/map/sets/cathedral.low.v2.glb", "characters/catpope/catpope_portrait-happy.v2.png");
    *stub.api.lock().unwrap() = answer(&files, low2, NEUTRAL, happy2, Some((low2, glb("low").len())));
    let sync = DeckSync::start(stub.config(&cache));
    sync.prepare(scene());
    settle(&sync, &scene());
    assert!(stub.hits("/cdn/").iter().any(|h| h.ends_with("cathedral.low.v2.glb")), "it was tried");
    assert_eq!(std::fs::read(cache.join("sets/cathedral/cathedral.low.glb")).unwrap(), glb("low"), "the old set stays");
    assert_eq!(std::fs::read(cache.join("portraits/catpope/catpope_portrait-happy.png")).unwrap(), png(), "the old sprite stays");
    assert!(!cache.join("sets/cathedral/cathedral.low.part").exists());
    let line = sync.line();
    assert!(line.starts_with("SYNC ERR"), "{line}");
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn offline_the_stage_plays_from_the_cache() {
    let cache = dir("offline");
    {
        let stub = stub_with_files();
        let sync = DeckSync::start(stub.config(&cache));
        sync.prepare(scene());
        settle(&sync, &scene());
    }
    // nothing listens there any more
    let gone = listen();
    let port = gone.local_addr().unwrap().port();
    drop(gone);
    let cfg = Config {
        api: format!("http://127.0.0.1:{port}/api"),
        cdn: format!("http://127.0.0.1:{port}/cdn"),
        cache: cache.clone(),
        sets: Some(cache.join("sets")),
        portraits: Some(cache.join("portraits")),
        token: deck_link::TokenSource::None,
        quality: "low".into(),
    };
    let sync = DeckSync::start(cfg);
    let t = Instant::now();
    sync.prepare(scene());
    settle(&sync, &scene());
    assert!(t.elapsed() < Duration::from_secs(8), "settled at once, not after timeouts: {:?}", t.elapsed());
    assert_eq!(sync.line(), "SYNC OFFLINE - FROM THE CACHE");
    assert!(cache.join("sets/cathedral/cathedral.low.glb").is_file(), "the cache is untouched");
    // a character wanted later doesn't wait either
    assert!(sync.wait(Want::Portrait { character: "yellowmouse".into(), mood: "neutral".into() }, PATIENCE));
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_character_wanted_later_is_asked_for_then() {
    let stub = stub_with_files();
    let cache = dir("later");
    let sync = DeckSync::start(stub.config(&cache));
    // a deck joins the conductor: its head asks for catpope's neutral sprite; no prepare named it
    assert!(sync.wait(Want::Portrait { character: "catpope".into(), mood: "neutral".into() }, PATIENCE));
    let api = stub.hits("/api/");
    assert_eq!(api.len(), 1);
    assert!(api[0].contains("sets=&characters=catpope"), "{}", api[0]);
    assert!(cache.join("portraits/catpope/catpope_portrait-neutral.png").is_file());
    // an unknown set is named an error, the stage keeps going
    *stub.api.lock().unwrap() = r#"{"sets":{},"characters":{},"missing":{"sets":["crypt"],"characters":[]}}"#.into();
    assert!(sync.wait(Want::Set { id: "crypt".into(), quality: "low".into() }, PATIENCE));
    let until = Instant::now() + PATIENCE;
    while sync.board.lock().unwrap().busy && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(sync.line(), "SYNC ERR SET CRYPT IS UNKNOWN TO THE API");
    let _ = std::fs::remove_dir_all(&cache);
}

/// the space side: the set loader's gate fetches before it reads, the heads' Source fetches then decodes
#[cfg(feature = "space")]
#[test]
fn the_space_node_s_loaders_go_through_the_sync() {
    use deck_native::space::portrait::Source;
    let stub = stub_with_files();
    let cache = dir("space");
    let sync = DeckSync::start(stub.config(&cache));
    let file = deck_native::cache::set_file(&cache.join("sets"), "cathedral", "low");
    assert!(!file.exists());
    (sync.gate())(&file);
    assert_eq!(std::fs::read(&file).unwrap(), glb("low"), "the gate returned once the file was there");
    let faces = sync.portraits().unwrap();
    assert_eq!(faces.load("catpope", "happy").unwrap().dimensions(), (4, 4));
    assert!(faces.load("catpope", "dizzy").unwrap_err().contains("catpope_portrait-dizzy"), "no sprite: the head falls back");
    let _ = std::fs::remove_dir_all(&cache);
}
