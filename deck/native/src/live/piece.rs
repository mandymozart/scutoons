//! The native deck's piece side (#91, #473): the core decides (deck-space's piece.rs `Side` – the same piece glides,
//! another loads beside the outlet, none goes back to the deck's scene; veil.rs fades). This file is only what the native
//! deck brings, on the frame thread:
//! - **find**: a built-in scene by its name (`scene:cathedral`), else `<cache>/pieces/<kind>.<id>.toml` (a scene, or a
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

/// What the deck has of a piece beyond its core rigs: a built-in scene by its name, else `<cache>/pieces/<kind>.<id>.toml`
/// (the cells here, a flash's check in flash.rs).
pub fn find(cache: &Path, place: &Place) -> Option<Found> {
    if place.kind == "scene" {
        if let Some(text) = crate::switcher::builtin(&place.id) {
            return Some(Found::Scene(text.to_string()));
        }
    }
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
