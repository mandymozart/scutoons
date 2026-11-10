//! The one resolver (#474, research/OPEN-WORLD.md §4 step 1, decision [resolver]): every value that falls back or is
//! overridden is asked here. In: a key and the sources that have an opinion on it, each with a strength. Out: the value,
//! where it came from (its origin), and what shows when that origin goes away (the fallback).
//!
//! - [`Strength`] – the order of strength, written here and nowhere else.
//! - [`Resolver`] – opinions per key (a stable id, never a position in a list) and source; per key and source one
//!   opinion, the newest replaces the one before.
//! - A timed change ([`Resolver::call`]) is one more source, one rule for every kind of entity and value: its length runs
//!   from its own start, when it ends the next source shows again. Time comes in as an argument; no clock in here.
//! - handler.rs – who answers an action: look → persona → character → the kind's default (SHOW.md §8).
//! - mapping.rs – a deck's control mappings: precoded < flashed < local (research/INPUT.md §6b).
//!
//! Not here (they decide in time, not by strength): the flash rule (show/flash.rs), the vote (relay/vote.rs).
pub mod handler;
pub mod mapping;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Who has an opinion, strongest first. **The declaration order is the order** – the one place it is written (OpenUSD's
/// LIVERPS taken over, research/OPEN-WORLD.md §2 #4); nothing else compares strengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strength {
    /// a timed change: called for a length, over every stored value, a deck's own local change too (owner 2026-11-10: a
    /// call is relay traffic that reaches every deck; a flash installs a new base and is what a deck can refuse)
    Timed,
    /// changed on this deck only (a learned knob, a source switched off), undoable by itself
    Local,
    /// what a flash brought (a show, its rigs and presets)
    Flashed,
    /// a member's draft, played on their own decks
    Draft,
    /// a character's look
    Look,
    /// a character's persona
    Persona,
    /// the entity itself: a character, or a location, item or scene at the same rank
    Character,
    /// the kind's default (`character:*`): OpenUSD's `specializes`, anything beats it
    #[serde(rename = "default")]
    KindDefault,
    /// what ships with the build: the floor
    Shipped,
}

impl Strength {
    /// every strength, strongest first
    pub const ALL: [Strength; 9] = [
        Strength::Timed,
        Strength::Local,
        Strength::Flashed,
        Strength::Draft,
        Strength::Look,
        Strength::Persona,
        Strength::Character,
        Strength::KindDefault,
        Strength::Shipped,
    ];

    /// its word on the wire and in an origin (`timed` … `default` … `shipped`)
    pub fn word(self) -> &'static str {
        match self {
            Strength::Timed => "timed",
            Strength::Local => "local",
            Strength::Flashed => "flashed",
            Strength::Draft => "draft",
            Strength::Look => "look",
            Strength::Persona => "persona",
            Strength::Character => "character",
            Strength::KindDefault => "default",
            Strength::Shipped => "shipped",
        }
    }

    pub fn parse(word: &str) -> Option<Strength> {
        Strength::ALL.into_iter().find(|s| s.word() == word)
    }
}

/// A source: its strength and its stable id (`character:catpope`, `deck:left`, the caller of a timed change …).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Source {
    pub strength: Strength,
    pub id: String,
}

impl Source {
    pub fn new(strength: Strength, id: &str) -> Source {
        Source { strength, id: id.to_string() }
    }
}

impl fmt::Display for Source {
    /// `persona character:catpope.mountaintiger`: inherited from where
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.strength.word(), self.id)
    }
}

/// One opinion as it was found: the value and its origin.
#[derive(Debug, PartialEq)]
pub struct Found<'a, V> {
    pub value: &'a V,
    pub origin: &'a Source,
}

impl<V> Clone for Found<'_, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V> Copy for Found<'_, V> {}

/// The answer for a key: the value, its origin, and the next opinion down (what shows when the origin goes away).
#[derive(Debug, PartialEq)]
pub struct Resolved<'a, V> {
    pub value: &'a V,
    pub origin: &'a Source,
    pub fallback: Option<Found<'a, V>>,
}

/// When a timed opinion holds: from `start`, for `length`, on the caller's clock (the same clock as `now`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: f64,
    pub length: f64,
}

impl Span {
    pub fn end(&self) -> f64 {
        self.start + self.length
    }

    pub fn holds(&self, now: f64) -> bool {
        now >= self.start && now < self.end()
    }
}

#[derive(Debug, Clone)]
struct Entry<V> {
    value: V,
    /// none = standing
    span: Option<Span>,
    /// the order it was given in: within one strength the newest wins
    seq: u64,
}

/// Opinions per key and source.
#[derive(Debug, Clone)]
pub struct Resolver<V> {
    keys: BTreeMap<String, BTreeMap<Source, Entry<V>>>,
    seq: u64,
}

impl<V> Default for Resolver<V> {
    fn default() -> Self {
        Resolver { keys: BTreeMap::new(), seq: 0 }
    }
}

impl<V> Resolver<V> {
    pub fn new() -> Resolver<V> {
        Resolver::default()
    }

    fn put(&mut self, key: &str, source: Source, value: V, span: Option<Span>) {
        self.seq += 1;
        let seq = self.seq;
        self.keys.entry(key.to_string()).or_default().insert(source, Entry { value, span, seq });
    }

    /// A standing opinion: per key and source one, the newest replaces the one before. A timed change goes through
    /// [`Resolver::call`].
    pub fn set(&mut self, key: &str, source: Source, value: V) {
        debug_assert!(source.strength != Strength::Timed, "a timed change is a call with a length");
        self.put(key, source, value, None);
    }

    /// A timed change by `caller`: from `start` for `length` it is an opinion of strength [`Strength::Timed`]. Per key and
    /// caller one: a second call replaces the first and its length runs from its own start. When it ends the next
    /// source shows again. The same rule for every key of every kind.
    pub fn call(&mut self, key: &str, caller: &str, value: V, start: f64, length: f64) {
        self.put(key, Source::new(Strength::Timed, caller), value, Some(Span { start, length }));
    }

    /// Take a source's opinion on a key back (`Reset this mapping`): the next source shows again.
    pub fn clear(&mut self, key: &str, source: &Source) -> Option<V> {
        let opinions = self.keys.get_mut(key)?;
        let gone = opinions.remove(source).map(|e| e.value);
        if opinions.is_empty() {
            self.keys.remove(key);
        }
        gone
    }

    /// Take every opinion of a strength back (a flash replaces the flashed layer; force defaults clears the local one).
    pub fn clear_strength(&mut self, strength: Strength) {
        for opinions in self.keys.values_mut() {
            opinions.retain(|s, _| s.strength != strength);
        }
        self.keys.retain(|_, o| !o.is_empty());
    }

    /// Drop the timed opinions that ended by `now` (they no longer count anyway).
    pub fn prune(&mut self, now: f64) {
        for opinions in self.keys.values_mut() {
            opinions.retain(|_, e| e.span.is_none_or(|s| now < s.end()));
        }
        self.keys.retain(|_, o| !o.is_empty());
    }

    /// The value for `key` at `now`, where it came from, and its fallback.
    pub fn resolve(&self, key: &str, now: f64) -> Option<Resolved<'_, V>> {
        self.resolve_where(key, now, |_| true)
    }

    /// The same, asked of the sources `counts` lets in (who answers: only the levels of one entity, handler.rs).
    pub fn resolve_where(&self, key: &str, now: f64, counts: impl Fn(&Source) -> bool) -> Option<Resolved<'_, V>> {
        let mut it = self.chain_where(key, now, counts).into_iter();
        let first = it.next()?;
        Some(Resolved { value: first.value, origin: first.origin, fallback: it.next() })
    }

    /// Every opinion on `key` that holds at `now`, strongest first: the whole ladder the editor shows.
    pub fn chain(&self, key: &str, now: f64) -> Vec<Found<'_, V>> {
        self.chain_where(key, now, |_| true)
    }

    fn chain_where(&self, key: &str, now: f64, counts: impl Fn(&Source) -> bool) -> Vec<Found<'_, V>> {
        let Some(opinions) = self.keys.get(key) else { return Vec::new() };
        let mut held: Vec<(&Source, &Entry<V>)> =
            opinions.iter().filter(|(s, e)| counts(s) && e.span.is_none_or(|sp| sp.holds(now))).collect();
        // the strength first; within one, the newest given
        held.sort_by(|a, b| a.0.strength.cmp(&b.0.strength).then(b.1.seq.cmp(&a.1.seq)));
        held.into_iter().map(|(origin, e)| Found { value: &e.value, origin }).collect()
    }

    /// The keys a strength has an opinion on (the flash report: the local changes per deck).
    pub fn keys_of(&self, strength: Strength) -> impl Iterator<Item = &str> {
        self.keys.iter().filter(move |(_, o)| o.keys().any(|s| s.strength == strength)).map(|(k, _)| k.as_str())
    }

    /// every key anyone has an opinion on
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.keys.keys().map(String::as_str)
    }
}
