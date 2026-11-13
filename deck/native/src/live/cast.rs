//! The native deck's playing scene (#517, research/SCENE-EVENTS.md §4 E2a): the core decides – deck-space published.rs
//! `Cast`, scene_play.rs `Playing`: the cast words of an `address`, the story events the scene's listeners hear. This file
//! only moves what the native deck brings, on the frame thread:
//! - **load**: the published scene the deck plays (`--scene scene:<id>[@v]`, deck.toml, F5) from its cache file
//!   (`<cache>/scenes/<id>.json`, published.rs) as the core's `Cast`; any other scene has none;
//! - **ask**: while the scene listens, the space's link asks the session for the readers' story (`deck.story`);
//! - **pass**: what came for it (deck-space relay.rs `In::Address`, `In::Story`) to the core, as it came;
//! - **switch**: a changed cast is a scene: its text written to `<cache>/live/cast.toml`, which the deck switches to like
//!   a piece (main.rs `change.ask`: a fade through black – the native deck has no in-place recast yet); the `Cast` stays
//!   the one that plays, so the next word changes the cast as it stands.
use deck_space::published::{Cast, SceneRef};
use deck_space::relay::In;
use deck_space::scene_play::Recast;
use std::path::Path;

/// where a changed cast's scene is written, under `<cache>`
pub const CAST_FILE: [&str; 2] = ["live", "cast.toml"];

/// the deck's playing scene, kept over the scene loads it asks for
#[derive(Default)]
pub struct Side {
    /// the scene argument the cast plays as (the `scene:<id>` it was loaded for, then the file it wrote) and the cast
    playing: Option<(String, Cast)>,
    /// the scene argument last seen: a new one loads its cast anew
    seen: Option<String>,
}

impl Side {
    /// The deck plays `scene` (its `--scene`): its cast from `<cache>/scenes/` when it is a published scene the cache
    /// keeps; the scene it switched to for a changed cast keeps the cast.
    pub fn scene(&mut self, cache: &Path, scene: Option<&str>) {
        if scene.map(str::to_string) == self.seen {
            return;
        }
        self.seen = scene.map(str::to_string);
        if self.playing.as_ref().is_some_and(|(plays_as, _)| Some(plays_as.as_str()) == scene) {
            return;
        }
        self.playing = scene.and_then(|s| Some((s.to_string(), load(cache, s)?)));
    }

    /// the playing scene's cast, if the deck plays a published scene
    pub fn cast(&self) -> Option<&Cast> {
        self.playing.as_ref().map(|(_, c)| c)
    }

    /// whether the playing scene listens to story events (the link asks for them while it does)
    pub fn listens(&self) -> bool {
        self.playing.as_ref().is_some_and(|(_, c)| c.playing().listens())
    }

    /// What came for the playing scene, to the core. Some(file) = the cast changed: the deck switches to that scene.
    pub fn take(&mut self, cache: &Path, came: Vec<In>) -> Option<String> {
        let (plays_as, cast) = self.playing.as_mut()?;
        let mut changed = false;
        for c in came {
            let did = match c {
                In::Address { address, action } => vec![cast.word(&address, &action)],
                In::Story(event) => cast.story(&event.into()),
                _ => Vec::new(),
            };
            for r in &did {
                tracing::info!("scene: {r:?} – {}", cast.playing().log().last().unwrap_or_default());
            }
            changed |= did.iter().any(|r| matches!(r, Recast::Changed { .. }));
        }
        if !changed {
            return None;
        }
        let file = cache.join(CAST_FILE[0]).join(CAST_FILE[1]);
        let written = cast.text().and_then(|text| {
            std::fs::create_dir_all(file.parent().unwrap_or(cache)).and_then(|_| std::fs::write(&file, text)).map_err(|e| format!("{}: {e}", file.display()))
        });
        if let Err(e) = written {
            tracing::warn!("scene: the changed cast is not played: {e}");
            return None;
        }
        *plays_as = file.to_string_lossy().into_owned();
        Some(plays_as.clone())
    }

    /// One frame: the scene the deck plays, its ask, what came for it through the space's link. Some(file) = switch to it.
    pub fn frame(&mut self, cache: &Path, scene: Option<&str>, space: Option<&mut deck_space::space::Space>) -> Option<String> {
        self.scene(cache, scene);
        let space = space?;
        space.ask_story(self.listens());
        let came = space.take_cast();
        self.take(cache, came)
    }
}

/// the cast of the published scene `scene` names, from the cache's copy (none: not a published scene, or not kept)
fn load(cache: &Path, scene: &str) -> Option<Cast> {
    let r = SceneRef::parse(scene)?.ok()?;
    let text = std::fs::read_to_string(cache.join("scenes").join(r.cache_name())).ok()?;
    Cast::from_cache(&text).map_err(|e| tracing::warn!("scene: {r}: no cast words or story while it plays: {e}")).ok()
}
