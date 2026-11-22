//! A rig that extends another (`extends = "character.default"`): the extended rig's tracks and routes play under its
//! own. `off` names extended tracks that are not compiled at all (they cost nothing); a track that `takes` an extended
//! track plays in its place. An extended route that touches an off or taken track drops with it. Folding happens before
//! a rig is checked as a whole (`Rig::extended`), so flatten, play and the scene resolver see one plain rig.
//! The default character rig (`character.default`, deck/native/rigs/default.toml): one simple character rig with a
//! built-in voice, the first iteration of any character; a character without a rig of its own plays it
//! (`Bundle::core_or_default`).
use super::address::{Address, Leaf};
use super::bundle::Bundle;
use super::{Library, Rig};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

/// the default character rig's subject
pub const DEFAULT_RIG: &str = "character.default";

impl Rig {
    /// the extended rig's tracks this rig turns off or takes over
    fn taken(&self) -> impl Iterator<Item = &str> {
        self.off.iter().map(String::as_str).chain(self.tracks.iter().filter_map(|t| t.takes.as_deref()))
    }

    /// what is wrong with the `extends` link alone (`Rig::check` lists it with the rest)
    pub(crate) fn extends_problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Some(subject) = &self.extends else {
            if self.taken().next().is_some() {
                out.push("'off' and 'takes' name tracks of an extended rig – this rig extends none".to_string());
            }
            return out;
        };
        let ext = match Bundle::core(subject) {
            Some(Ok(b)) => b,
            Some(Err(e)) => return vec![format!("the extended rig {subject}: {e:#}")],
            None => return vec![format!("extends '{subject}', not a core rig of the deck")],
        };
        if ext.rig.extends.is_some() || *subject == self.subject {
            out.push(format!("extends '{subject}', which extends a rig itself – an extended rig stands on its own"));
        }
        let has = |id: &str| ext.rig.tracks.iter().any(|t| t.id == id);
        for t in self.taken().filter(|t| !has(t)) {
            out.push(format!("the extended rig {subject} has no track '{t}'"));
        }
        let taken: Vec<&str> = self.taken().collect();
        for t in self.tracks.iter().filter(|t| has(&t.id) && !taken.contains(&t.id.as_str())) {
            out.push(format!("track '{}' is the extended rig's too – take it over ('takes') or name it apart", t.id));
        }
        out
    }

    /// The rig with the rig it extends folded in (that one's kept tracks first, its kept routes before its own) and `lib`
    /// with that one's instruments and presets; None when it extends none.
    pub fn extended(&self, lib: &Library) -> Result<Option<(Rig, Library)>> {
        let Some(subject) = &self.extends else { return Ok(None) };
        if let Some(e) = self.extends_problems().into_iter().next() {
            bail!("rig {}: {e}", self.subject);
        }
        let ext = Bundle::core(subject).context("a checked extended rig")??;
        let taken: Vec<&str> = self.taken().collect();
        let kept = |track: &str| !taken.contains(&track);
        let route_kept = |end: &str| match Address::leaf(end) {
            Ok(Leaf::Port { track: Some(t), .. }) => kept(&t),
            _ => true,
        };
        let mut rig = self.clone();
        rig.extends = None;
        rig.off.clear();
        rig.tracks = ext.rig.tracks.iter().filter(|t| kept(&t.id)).cloned().chain(self.tracks.iter().cloned()).collect();
        for t in &mut rig.tracks {
            t.takes = None;
        }
        rig.routes = ext.rig.routes.iter().filter(|r| route_kept(&r.from) && route_kept(&r.to)).cloned().chain(self.routes.iter().cloned()).collect();
        // the extended rig's moods and personas fill what the rig leaves out
        for (own, theirs) in [(&mut rig.moods, &ext.rig.moods), (&mut rig.personas, &ext.rig.personas)] {
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
        let used: Vec<&str> = ext.rig.tracks.iter().filter(|t| kept(&t.id)).map(|t| t.instrument.as_str()).collect();
        let mut instruments = lib.instruments.clone();
        for i in ext.instruments.iter().filter(|i| used.contains(&i.id.as_str())) {
            instruments.entry(i.id.clone()).or_insert_with(|| i.clone());
        }
        let mut presets: BTreeMap<_, _> = lib.presets.presets.clone();
        for p in ext.presets.iter().filter(|p| used.contains(&p.instrument.as_str())) {
            presets.entry(p.id.clone()).or_insert_with(|| p.clone());
        }
        let lib = Library::new(instruments.into_values(), presets.into_values()).with_modules(lib.modules.values().cloned());
        Ok(Some((rig, lib)))
    }
}

impl Bundle {
    /// The bundle with the rig it extends folded in (`Rig::extended`): what plays. Itself when it extends none.
    pub fn extended(&self) -> Result<std::borrow::Cow<'_, Bundle>> {
        let Some((rig, lib)) = self.rig.extended(&self.library())? else { return Ok(std::borrow::Cow::Borrowed(self)) };
        let mut b = self.clone();
        b.rig = rig;
        b.instruments = lib.instruments.into_values().collect();
        b.presets = lib.presets.presets.into_values().collect();
        Ok(std::borrow::Cow::Owned(b))
    }

    /// What a character without a rig of its own plays: a rig of its own subject that extends the default character rig
    /// and adds nothing
    pub fn fallback(subject: &str) -> Bundle {
        let mut b = Bundle::empty(subject);
        b.rig.extends = Some(DEFAULT_RIG.to_string());
        b
    }

    /// The core rig of `subject`, else for a character the default character rig as its fallback (`fallback`); None for
    /// anything else the deck carries no rig of
    pub fn core_or_default(subject: &str) -> Option<Result<Bundle>> {
        Bundle::core(subject).or_else(|| subject.strip_prefix("character.").filter(|c| !c.is_empty()).map(|_| Ok(Bundle::fallback(subject))))
    }
}

#[cfg(test)]
mod tests;
