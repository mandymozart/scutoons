//! The native deck's piece side (#91): the outlet's cells (deck-outlet's route.rs `Routed::Piece`) go to deck-space's
//! piece.rs, which decides – the same piece glides, another loads, none fades to black (#467). This adapter only moves what
//! it says, on the frame thread:
//! - **Load**: the piece's whole scene beside the outlet (`piece::beside`: the outlet and its screen kept, the Load's
//!   values set) is written to `<cache>/live/piece.toml` and played by the deck's switch – the switcher's fade, then the
//!   scene loads; the outlet joins again and its snapshot's cell answers "the same piece".
//! - **Black**: the outlet scene again (`outlet`, built in).
//! - **Glide**: the gliding values into the control domain each frame (rig.rs `set`, as a mood's replay).
//!
//! What the deck has beyond its core rigs (`find`): a built-in scene by its name (`scene:cathedral`), else
//! `<cache>/pieces/<kind>.<id>.toml` (a scene, or a rig bundle). The HUD's line is deck-outlet's `Status::set_piece`.
use super::Live;
use crate::control::Controls;
use crate::graph::Graph;
use deck_space::piece::{beside, Found, Pieces, What};
use deck_space::rig::address::Place;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct Side {
    pieces: Pieces,
    /// `<cache>`: its `pieces/` are read, its `live/piece.toml` written
    cache: PathBuf,
    started: Instant,
}

impl Default for Side {
    fn default() -> Side {
        Side::new()
    }
}

impl Side {
    pub fn new() -> Side {
        Side { pieces: Pieces::new(), cache: crate::cache::default_dir(), started: Instant::now() }
    }

    fn find(&self, place: &Place) -> Option<Found> {
        find(&self.cache, place)
    }

    /// One frame: the cells the outlet routed, then the glides. Some(scene) = the deck switches to it (a file, or `outlet`).
    pub fn frame(&mut self, live: &Live, graph: &Graph, ctl: &mut Controls) -> Option<String> {
        let now = self.started.elapsed().as_secs_f64() * 1000.0;
        let mut switch = None;
        while let Ok(cell) = live.pieces.try_recv() {
            let mut pieces = std::mem::take(&mut self.pieces);
            let answer = pieces.cell(&cell, now, |p| self.find(p));
            self.pieces = pieces;
            let did = self.pieces.log().filter(|l| !answer.problems.iter().any(|p| p == l)).last().unwrap_or_default().to_string();
            tracing::info!("piece: {did}");
            for p in &answer.problems {
                tracing::warn!("piece: {p}");
            }
            live.status.lock().unwrap().set_piece(&did, &answer.problems);
            switch = match answer.what {
                What::Load { values, .. } => self.load(&values).or(switch),
                What::Black { .. } => Some("outlet".to_string()),
                _ => switch,
            };
        }
        for (path, v) in self.pieces.values_at(now) {
            // a value whose node the scene lacks was said when the cell came (piece.rs `resolve`)
            crate::rig::set(graph, ctl, &path, v);
        }
        switch
    }

    /// the piece's scene beside the outlet, written for the switch
    fn load(&self, values: &[(String, f32)]) -> Option<String> {
        let scene = self.pieces.played()?.scene.clone();
        let text = beside(&scene, deck_space::scene::OUTLET_SCENE, values).map_err(|e| tracing::warn!("piece: {e}")).ok()?;
        let file = self.cache.join("live").join("piece.toml");
        std::fs::create_dir_all(file.parent()?).ok()?;
        std::fs::write(&file, text).map_err(|e| tracing::warn!("piece: {}: {e}", file.display())).ok()?;
        Some(file.to_string_lossy().into_owned())
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
