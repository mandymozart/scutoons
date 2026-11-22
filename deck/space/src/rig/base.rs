//! The base rig (`character.base`, deck/native/rigs/base.toml): one simple character rig with a built-in voice – the
//! first iteration of any character and the lasting rig of those that never become scutoons; a character without a rig
//! of its own plays it (`Bundle::core_or_base`).
//! A rig builds on it with `base = "character.base"`: the base's tracks and routes play under its own. `off` names base
//! tracks that are not compiled at all (they cost nothing); a track that `takes` a base track plays in its place. A
//! base route that touches an off or taken track drops with it. Folding happens before a rig is checked as a whole
//! (`Rig::based`), so flatten, play and the scene resolver see one plain rig.
use super::address::{Address, Leaf};
use super::bundle::Bundle;
use super::{Library, Rig};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

/// the base rig's subject
pub const BASE: &str = "character.base";

impl Rig {
    /// the base tracks this rig turns off or takes over
    fn taken(&self) -> impl Iterator<Item = &str> {
        self.off.iter().map(String::as_str).chain(self.tracks.iter().filter_map(|t| t.takes.as_deref()))
    }

    /// what is wrong with the base link alone (`Rig::check` lists it with the rest)
    pub(crate) fn base_problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Some(subject) = &self.base else {
            if self.taken().next().is_some() {
                out.push("'off' and 'takes' name tracks of a base rig – this rig names no base".to_string());
            }
            return out;
        };
        let base = match Bundle::core(subject) {
            Some(Ok(b)) => b,
            Some(Err(e)) => return vec![format!("the base rig {subject}: {e:#}")],
            None => return vec![format!("base '{subject}' is not a core rig of the deck")],
        };
        if base.rig.base.is_some() || *subject == self.subject {
            out.push(format!("base '{subject}' builds on another rig – a base rig stands on its own"));
        }
        let has = |id: &str| base.rig.tracks.iter().any(|t| t.id == id);
        for t in self.taken().filter(|t| !has(t)) {
            out.push(format!("the base rig {subject} has no track '{t}'"));
        }
        let taken: Vec<&str> = self.taken().collect();
        for t in self.tracks.iter().filter(|t| has(&t.id) && !taken.contains(&t.id.as_str())) {
            out.push(format!("track '{}' is the base rig's too – take it over ('takes') or name it apart", t.id));
        }
        out
    }

    /// The rig with its base folded in (the base's kept tracks first, its kept routes before its own) and `lib` with the
    /// base's instruments and presets; None when it names no base.
    pub fn based(&self, lib: &Library) -> Result<Option<(Rig, Library)>> {
        let Some(subject) = &self.base else { return Ok(None) };
        if let Some(e) = self.base_problems().into_iter().next() {
            bail!("rig {}: {e}", self.subject);
        }
        let base = Bundle::core(subject).context("a checked base")??;
        let taken: Vec<&str> = self.taken().collect();
        let kept = |track: &str| !taken.contains(&track);
        let route_kept = |end: &str| match Address::leaf(end) {
            Ok(Leaf::Port { track: Some(t), .. }) => kept(&t),
            _ => true,
        };
        let mut rig = self.clone();
        rig.base = None;
        rig.off.clear();
        rig.tracks = base.rig.tracks.iter().filter(|t| kept(&t.id)).cloned().chain(self.tracks.iter().cloned()).collect();
        for t in &mut rig.tracks {
            t.takes = None;
        }
        rig.routes = base.rig.routes.iter().filter(|r| route_kept(&r.from) && route_kept(&r.to)).cloned().chain(self.routes.iter().cloned()).collect();
        // the base's moods and personas fill what the rig leaves out
        for (own, theirs) in [(&mut rig.moods, &base.rig.moods), (&mut rig.personas, &base.rig.personas)] {
            for (name, v) in theirs {
                let mine = own.entry(name.clone()).or_default();
                for (track, p) in v.presets.iter().filter(|(t, _)| kept(t)) {
                    mine.presets.entry(track.clone()).or_insert_with(|| p.clone());
                }
                for (path, x) in v.values.iter().filter(|(path, _)| route_kept(path)) {
                    mine.values.entry(path.clone()).or_insert(*x);
                }
            }
        }
        let used: Vec<&str> = base.rig.tracks.iter().filter(|t| kept(&t.id)).map(|t| t.instrument.as_str()).collect();
        let mut instruments = lib.instruments.clone();
        for i in base.instruments.iter().filter(|i| used.contains(&i.id.as_str())) {
            instruments.entry(i.id.clone()).or_insert_with(|| i.clone());
        }
        let mut presets: BTreeMap<_, _> = lib.presets.presets.clone();
        for p in base.presets.iter().filter(|p| used.contains(&p.instrument.as_str())) {
            presets.entry(p.id.clone()).or_insert_with(|| p.clone());
        }
        let lib = Library::new(instruments.into_values(), presets.into_values()).with_modules(lib.modules.values().cloned());
        Ok(Some((rig, lib)))
    }
}

impl Bundle {
    /// The bundle with its base folded in (`Rig::based`): what plays. Itself when it names no base.
    pub fn based(&self) -> Result<std::borrow::Cow<'_, Bundle>> {
        let Some((rig, lib)) = self.rig.based(&self.library())? else { return Ok(std::borrow::Cow::Borrowed(self)) };
        let mut b = self.clone();
        b.rig = rig;
        b.instruments = lib.instruments.into_values().collect();
        b.presets = lib.presets.presets.into_values().collect();
        Ok(std::borrow::Cow::Owned(b))
    }

    /// What a character without a rig of its own plays: a rig of its own subject that is the base rig and nothing more
    pub fn fallback(subject: &str) -> Bundle {
        let mut b = Bundle::empty(subject);
        b.rig.base = Some(BASE.to_string());
        b
    }

    /// The core rig of `subject`, else for a character the base rig as its fallback (`fallback`); None for anything else
    /// the deck carries no rig of
    pub fn core_or_base(subject: &str) -> Option<Result<Bundle>> {
        Bundle::core(subject).or_else(|| subject.strip_prefix("character.").filter(|c| !c.is_empty()).map(|_| Ok(Bundle::fallback(subject))))
    }
}

#[cfg(test)]
mod tests;
