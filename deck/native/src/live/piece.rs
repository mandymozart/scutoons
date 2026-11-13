//! The native deck's piece side (#91, #473): the core decides (deck-space's piece.rs `Side` – the same piece glides,
//! another loads beside the outlet, none goes back to the deck's scene; veil.rs fades). This file is only what the native
//! deck brings, on the frame thread:
//! - **find**: a system scene by its name (`scene:outlet`; never a sandbox, #427), else `<cache>/pieces/<kind>.<id>.toml` (a scene, or a
//!   rig bundle);
//! - **switch**: the deck's switch takes a name or a file, so the scene's text is written to `<cache>/live/piece.toml`;
//!   the deck's own scene is `outlet` (built in). The outlet joins again and its snapshot's cell answers "the same piece";
//! - **set**: a gliding value into the control domain (deck-engine's `Controls::set_path`).
//!
//! The HUD's line is deck-outlet's `Status::set_piece`.
use super::Live;
use crate::control::Controls;
use crate::graph::Graph;
use deck_space::piece::{Found, Switch};
use deck_space::published::SceneRef;
use deck_space::rig::address::Place;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct Side {
    side: deck_space::piece::Side,
    /// `<cache>`: its `pieces/` are read, its `live/piece.toml` written
    cache: PathBuf,
    started: Instant,
}

impl Default for Side {
    fn default() -> Side {
        Side::new()
    }
}

/// What the deck has of a piece beyond its core rigs: a built-in scene by its name, else a published scene the cache keeps
/// (#514: `scene:<id>`, offline through the core's loader), else `<cache>/pieces/<kind>.<id>.toml` (the cells here, a
/// flash's check in flash.rs). A published scene that does not play is the core's reason, as a missing piece.
pub fn find(cache: &Path, place: &Place) -> Option<Found> {
    if place.kind == "scene" {
        if let Some(text) = crate::switcher::builtin(&place.id) {
            return Some(Found::Scene(text.to_string()));
        }
        return Some(match published_copy(cache, &place.id) {
            Ok(text) => Found::Scene(text),
            Err(why) => file(cache, place).unwrap_or(Found::Missing(why)),
        });
    }
    file(cache, place)
}

/// The published scene `id` from its cache copy (`<cache>/scenes/`): the core's loader with no network asked, so the copy
/// plays and a scene that is not cached says why (the loader's words).
fn published_copy(cache: &Path, id: &str) -> Result<String, String> {
    let r = match SceneRef::parse(&format!("scene:{id}")) {
        Some(Ok(r)) => r,
        Some(Err(why)) => return Err(why),
        None => return Err(format!("scene:{id} names no scene")),
    };
    let loaded = crate::published::load(r, &cache.join("scenes"), |_| (0, String::new())).map_err(|e| format!("{e:#}"))?;
    loaded.text()
}

/// `<cache>/pieces/<kind>.<id>.toml` as the piece it is
fn file(cache: &Path, place: &Place) -> Option<Found> {
    let text = std::fs::read_to_string(cache.join("pieces").join(format!("{}.{}.toml", place.kind, place.id))).ok()?;
    Some(if place.kind == "scene" { Found::Scene(text) } else { Found::Rig(text) })
}

impl Side {
    pub fn new() -> Side {
        Side { side: deck_space::piece::Side::new(deck_space::scene::OUTLET_SCENE), cache: crate::cache::default_dir(), started: Instant::now() }
    }

    /// One frame: the cells the outlet routed, then the glides. Some(scene) = the deck switches to it (a file, or `outlet`).
    pub fn frame(&mut self, live: &Live, graph: &Graph, ctl: &mut Controls) -> Option<String> {
        let now = self.started.elapsed().as_secs_f64() * 1000.0;
        let mut switch = None;
        while let Ok(cell) = live.pieces.try_recv() {
            let said = self.side.cell(&cell, now, |p| find(&self.cache, p));
            tracing::info!("piece: {}", said.did);
            for p in &said.problems {
                tracing::warn!("piece: {p}");
            }
            live.status.lock().unwrap().set_piece(&said.did, &said.problems);
            switch = match said.switch {
                Some(Switch::Scene(text)) => self.write(&text).or(switch),
                Some(Switch::Deck) => Some("outlet".to_string()),
                None => switch,
            };
        }
        for (path, v) in self.side.values_at(now) {
            // a value whose node the scene lacks was said when the cell came (piece.rs `resolve`)
            ctl.set_path(graph, &path, v);
        }
        switch
    }

    /// the scene's text as a file for the switch
    fn write(&self, text: &str) -> Option<String> {
        let [dir, name] = crate::switcher::PIECE_FILE;
        let file = self.cache.join(dir).join(name);
        std::fs::create_dir_all(file.parent()?).ok()?;
        std::fs::write(&file, text).map_err(|e| tracing::warn!("piece: {}: {e}", file.display())).ok()?;
        Some(file.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_space::piece::available;
    use std::path::PathBuf;

    const NAVE: &str = include_str!("../../../../server/tests/fixtures/scenes/nave.json");
    const RIGS: &str = include_str!("../../../../server/tests/fixtures/rigs/scene-content.json");

    fn dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("deck-piece-{name}-{}", std::process::id()))
    }

    /// the API with `scene:nave` published at version 3 (its document and scene-content)
    fn api(path: &str) -> (u16, String) {
        if path.starts_with("/scenes/nave") {
            let doc = NAVE.replacen("\"format\": 2,", "\"format\": 2, \"version\": 3,", 1);
            (200, format!("{{\"status\":\"success\",\"scene\":{doc}}}"))
        } else {
            (200, RIGS.to_string())
        }
    }

    /// #514: a cell names `scene:nave`, the cache holds it, the cell plays it with the API unreachable
    #[test]
    fn a_cell_plays_a_cached_published_scene_offline() {
        let cache = dir("cached");
        let _ = std::fs::remove_dir_all(&cache);
        crate::published::load(SceneRef::parse("scene:nave").unwrap().unwrap(), &cache.join("scenes"), api).unwrap();
        let found = find(&cache, &Place::new("scene", "nave"));
        assert!(matches!(found, Some(Found::Scene(_))), "the cache's copy is found");
        assert_eq!(available("scene:nave", |p| find(&cache, p)), Ok(()), "the core plays it, no network asked");
        let _ = std::fs::remove_dir_all(&cache);
    }

    /// #514: a published scene the cache does not keep: the core's reason, as for any missing piece
    #[test]
    fn a_published_scene_not_cached_gives_the_core_s_reason() {
        let cache = dir("uncached");
        let _ = std::fs::remove_dir_all(&cache);
        let err = available("scene:nave", |p| find(&cache, p)).unwrap_err();
        assert_eq!(err, "scene:nave: the API can't be reached and this deck has no copy");
        let _ = std::fs::remove_dir_all(&cache);
    }
}
