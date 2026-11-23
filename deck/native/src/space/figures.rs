//! The space's loaders on the deck – the heads' and cards' pictures, the set's gate, the cast's own 3D models
//! (deck-space figure.rs) – through the sync (it fetches, then the folder is read), else the local folder as it is;
//! and what the API answered for the scene, handed to the space each time it is new (`Space::set_looks`: each look's
//! form and model). Only bytes and answers move here; what draws a member is the core's.
use super::figure::{self, Figures};
use super::portrait::Portraits;
use std::path::Path;

/// how long the figures thread waits for a character's model (a few MB)
#[cfg(feature = "sync")]
pub const FIGURE_WAIT: std::time::Duration = std::time::Duration::from_secs(120);

/// the local folder's loaders (`--portraits`, or a deck without the sync)
pub fn local(faces: &Path) -> (Portraits, Option<super::Gate>, Figures) {
    (Portraits::local(faces), None, Figures::new(Box::new(figure::LocalDir(faces.to_path_buf()))))
}

/// the loaders through the sync when there is one and it fetches into the folder; else `local`
#[cfg(feature = "sync")]
pub fn loaders(sync: Option<&crate::sync::DeckSync>, faces: &Path) -> (Portraits, Option<super::Gate>, Figures) {
    let Some(y) = sync else { return local(faces) };
    let (portraits, figures) = match (y.portraits(), y.portraits.clone()) {
        (Some(p), Some(dir)) => {
            let wait = y.clone();
            let models = figure::Arriving { dir: figure::LocalDir(dir), wait: move |file: &figure::Ask| match file {
                figure::Ask::Character(c) => {
                    wait.wait(crate::sync::Want::Model { character: c.clone() }, FIGURE_WAIT);
                    // #817: its idle (a library animation) – settled at once when the answer names none
                    wait.wait(crate::sync::Want::Idle { character: c.clone() }, FIGURE_WAIT);
                }
                // a rig's body: a file by its CDN path, fetched as any stage media
                figure::Ask::Body(path) => {
                    wait.wait(crate::sync::Want::Media { path: path.clone() }, FIGURE_WAIT);
                }
            } };
            (Portraits::new(Box::new(p)), Figures::new(Box::new(models)))
        }
        _ => (Portraits::local(faces), Figures::new(Box::new(figure::LocalDir(faces.to_path_buf())))),
    };
    (portraits, Some(y.gate()), figures)
}

/// what the API answered since the last frame, to the space's cast
#[cfg(feature = "sync")]
pub fn looks(sync: Option<&crate::sync::DeckSync>, space: &mut super::Space) {
    let content = sync.and_then(|y| y.answer.lock().ok()?.looks.take());
    if let Some(c) = content {
        space.set_looks(&c);
    }
}
