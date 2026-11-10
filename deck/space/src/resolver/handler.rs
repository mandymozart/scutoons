//! Who answers an action (SHOW.md §8, decision [events]): the receiver looks for a handler at its look, persona,
//! character, then the kind's default – the resolver's order, not a second one. Nobody knows: no answer, the caller logs
//! it. Who picks up (an address going down, `Target::picks_up`) is not asked here: a specific address that finds nobody
//! does not go up.
//!
//! No caller yet (#474 slice 2): no document holds handlers today; the entity document (CATALOG-LINK §7a) and the show's
//! events will fill [`Handlers`].
use super::{Resolved, Resolver, Source, Strength};
use crate::rig::address::Target;

/// The source a handler held at `at` speaks with: `character:*` the kind's default, `character:catpope` the character
/// (or any entity), `.mountaintiger` its persona, `.ghost` its look.
pub fn level(at: &Target) -> Source {
    let strength = match (&at.id, at.below.len()) {
        (None, _) => Strength::KindDefault,
        (Some(_), 0) => Strength::Character,
        (Some(_), 1) => Strength::Persona,
        _ => Strength::Look,
    };
    Source::new(strength, &at.to_string())
}

/// The levels that may answer for `who`, the kind's default first: `character:*`, `character:catpope`,
/// `character:catpope.mountaintiger`, `character:catpope.mountaintiger.ghost`. None for `*`: an answer comes from one.
pub fn levels(who: &Target) -> Option<Vec<Source>> {
    who.id.as_ref()?;
    let mut out = vec![level(&Target { kind: who.kind.clone(), id: None, below: vec![] })];
    for depth in 0..=who.below.len() {
        out.push(level(&Target { kind: who.kind.clone(), id: who.id.clone(), below: who.below[..depth].to_vec() }));
    }
    Some(out)
}

/// Handlers per action (`close_doors`, `dance:samba`) and the level that holds them.
#[derive(Debug, Clone)]
pub struct Handlers<H> {
    held: Resolver<H>,
}

impl<H> Default for Handlers<H> {
    fn default() -> Self {
        Handlers { held: Resolver::new() }
    }
}

impl<H> Handlers<H> {
    pub fn new() -> Handlers<H> {
        Handlers::default()
    }

    /// `at` handles `action` (one handler per level and action; a second replaces the first). `at` = `character:*` is
    /// the kind's default.
    pub fn hold(&mut self, at: &Target, action: &str, handler: H) {
        self.held.set(action, level(at), handler);
    }

    pub fn release(&mut self, at: &Target, action: &str) -> Option<H> {
        self.held.clear(action, &level(at))
    }

    /// Who answers `action` for `who` (`character:catpope.mountaintiger.ghost`): the handler, the level it was found at,
    /// and the one below it. None: nobody knows – nothing happens, the caller logs it.
    pub fn answer(&self, who: &Target, action: &str) -> Option<Resolved<'_, H>> {
        let levels = levels(who)?;
        // handlers are standing opinions: the time does not matter
        self.held.resolve_where(action, 0.0, |s| levels.contains(s))
    }
}
